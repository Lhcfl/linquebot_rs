/*! Datasets for working with the chats

## About the file-based dataset

The file uses a block-based format. Each block has layout [n elem, elem index, elem data].
There's always an extra index in the elem index in each block pointing to the pass-the-end position of the element data,
which simplifies the read and serves as a pointer to the next block.
The n elem field is usize, elem indices are u64, and the elem data are u32 token data.
They're designed to be natively endianed to avoid translation cost.

**Note** that the designs of the file format prevents the dataset be shared among different machines:
Just share the original chat data and recreate the dataset file.
*/
use std::{
    io::{Read, Seek, Write},
    ops::RangeBounds,
    path::Path,
};

use anyhow::Context;
use burn::data::dataset::Dataset;
use memmap2::Mmap;
use num_traits::{Euclid, FromBytes};
use rand::RngExt;
use tokenizers::Tokenizer;

const BLOCK_SIZE: usize = 2048 - 1;

/// A writer to the chat database supporting appending and shrinking (by removing blocks from the head)
#[derive(Debug)]
pub struct ChatWriter {
    fd: std::fs::File,
    tokenizer: Tokenizer,
    n_elem: usize,
    cur_block: u64,
}

impl ChatWriter {
    pub fn new(path: impl AsRef<Path>, tokenizer: Tokenizer) -> anyhow::Result<Self> {
        let mut fd = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(path)
            .context("open data file")?;
        // Special case: no blocks
        if fd.metadata()?.len() == 0 {
            let mut this = Self { fd, tokenizer, n_elem: 0, cur_block: 0 };
            this.add_block()?;
            return Ok(this);
        }
        // Iterate blocks to find the last one (with less than BLOCK_SIZE elements)
        let (n_elem, cur_block) = loop {
            let n_elem = usize::from_ne_bytes(fd.read_array().context("read block size")?);
            if n_elem < BLOCK_SIZE {
                break (n_elem, fd.stream_position()? - size_of::<usize>() as u64);
            }
            // After reading n_elem, fd at offsets, read offsets[BLOCK_SIZE]
            fd.seek(std::io::SeekFrom::Current((BLOCK_SIZE * size_of::<u64>()) as i64))?;
            let offset = u64::from_ne_bytes(fd.read_array().context("read next block index")?);
            fd.seek(std::io::SeekFrom::Start(offset))?;
        };
        // Contract of [`add`]: every time when it's called, the cursor should be at EOF
        fd.seek(std::io::SeekFrom::End(0))?;
        Ok(Self { fd, tokenizer, n_elem, cur_block })
    }

    pub fn into_dataset(mut self) -> anyhow::Result<ChatFile> {
        self.fd.flush()?;
        ChatFile::from_fd(self.fd)
    }

    fn add_block(&mut self) -> anyhow::Result<()> {
        self.cur_block = self.fd.stream_position()?;
        self.n_elem = 0;
        let target_len = self.fd.metadata()?.len()
            + (size_of::<usize>() + (BLOCK_SIZE + 1) * size_of::<u64>()) as u64;
        self.fd.set_len(target_len)?;
        // n elem (0) and the first boundary
        self.fd.write(&[0_usize.to_ne_bytes(), target_len.to_ne_bytes()].concat())?;
        self.fd.seek(std::io::SeekFrom::End(0))?;
        Ok(())
    }

    pub fn add(&mut self, item: &str) -> anyhow::Result<()> {
        // Convert to tokens
        let item = self.tokenizer.encode(item, true).map_err(anyhow::Error::from_boxed)?;
        let item = item.get_ids();

        if self.n_elem == BLOCK_SIZE {
            self.add_block()?;
        }

        // Since we're already at EOF, write the actual data first :)
        // SAFETY: u32 is strictly aligned to u8, and it'll be read with native endian,
        // effectively transmuting
        self.fd.write(&unsafe { item.align_to::<u8>() }.1)?;
        let pos = self.fd.stream_position()?;
        self.n_elem += 1;

        // Write metadata: modify len and append end pos: start pos is written by previous item
        self.fd.seek(std::io::SeekFrom::Start(self.cur_block))?;
        self.fd.write(&self.n_elem.to_ne_bytes())?;
        // goto offsets[n_elem]
        self.fd.seek(std::io::SeekFrom::Current((self.n_elem * size_of::<u64>()) as i64))?;
        self.fd.write(&pos.to_ne_bytes())?;
        self.fd.seek(std::io::SeekFrom::End(0))?;

        Ok(())
    }
}

/// Dataset corresponding to the chat file
///
/// Offsets are just read from the file, since OS cache is good enough.
#[derive(Debug)]
pub struct ChatFile {
    mmap: Mmap,
    n: usize,
    blocks: Vec<usize>,
}

impl ChatFile {
    pub fn from_fd(fd: std::fs::File) -> anyhow::Result<Self> {
        let mmap = unsafe { Mmap::map(&fd) }.context("failed to mmap data file")?;

        // Special case: empty (why?
        if mmap.len() == 0 {
            return Ok(Self { mmap, n: 0, blocks: vec![] });
        }

        let mut n = 0;
        let mut blocks = vec![];
        let mut cursor = 0;
        loop {
            let cur = Self::read_num::<_, usize>(&mmap, cursor)?;
            n += cur;
            blocks.push(cursor);
            if cur != BLOCK_SIZE {
                break;
            }
            // offset[BLOCK_SIZE] for the address of next block
            cursor = Self::read_num::<_, u64>(
                &mmap,
                cursor + size_of::<usize>() + BLOCK_SIZE * size_of::<u64>(),
            )? as usize;
        }

        Ok(Self { mmap, n, blocks })
    }

    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let fd = std::fs::File::open(path).context("failed to open data file")?;
        Self::from_fd(fd)
    }

    fn read_num<const D: usize, T: FromBytes<Bytes = [u8; D]>>(
        mmap: &Mmap,
        offset: usize,
    ) -> anyhow::Result<T> {
        let bytes = &mmap[offset..offset + std::mem::size_of::<T>()];
        Ok(T::from_ne_bytes(&bytes.try_into().context("invalid usize bytes")?))
    }

    /// Get the absolute offset of item at idx, accessing the special ending position if `ending` is true.
    fn offset_of(&self, idx: usize, ending: bool) -> anyhow::Result<usize> {
        if (idx == self.n && !ending) || idx > self.n {
            return Err(anyhow::anyhow!("out of bound index of {idx}"));
        }
        let (blk, idx) = idx.div_rem_euclid(&BLOCK_SIZE);
        let (blk, idx) = if ending && idx == 0 {
            (blk.checked_sub(1).context("(0, 0) for ending")?, BLOCK_SIZE)
        } else {
            (blk, idx)
        };
        Self::read_num(&self.mmap, self.blocks[blk] + size_of::<usize>() + idx * size_of::<u64>())
    }

    fn get_range(&self, range: impl RangeBounds<usize>) -> Option<Vec<u32>> {
        let start = match range.start_bound() {
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(i) => *i + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(i) => *i + 1,
            std::ops::Bound::Excluded(i) => *i,
            std::ops::Bound::Unbounded => self.n,
        };
        // Transform the range into intervals within the same block to reduce reading
        let chunks = std::iter::from_coroutine(
            #[coroutine]
            || {
                let mut prev = start;
                let mut front = ((start / BLOCK_SIZE) + 1) * BLOCK_SIZE;
                while front < end {
                    yield (prev, front);
                    prev = front;
                    front += BLOCK_SIZE;
                }
                yield (prev, end);
            },
        );
        chunks
            .flat_map(|(start, end)| {
                std::iter::from_coroutine(
                    #[coroutine]
                    move || {
                        // Map intervals into offsets
                        let Some(start) = self.offset_of(start, false).ok() else {
                            yield None;
                            return;
                        };
                        let Some(end) = self.offset_of(end, true).ok() else {
                            yield None;
                            return;
                        };
                        // Yield the chunks
                        let chunks =
                            self.mmap[start as usize..end as usize].chunks(size_of::<u32>());
                        for bs in chunks {
                            let Some(bs) = bs.as_array() else {
                                yield None;
                                return;
                            };
                            yield Some(u32::from_ne_bytes(*bs));
                        }
                    },
                )
            })
            .collect()
    }
}

impl Dataset<Vec<u32>> for ChatFile {
    fn get(&self, idx: usize) -> Option<Vec<u32>> {
        self.get_range(idx..=idx)
    }

    fn len(&self) -> usize {
        self.n
    }
}

#[derive(Debug)]
pub struct SeqLenWrapper {
    inner: ChatFile,
    n: usize,
    seq_len: usize,
}

impl SeqLenWrapper {
    pub fn new(inner: ChatFile, seq_len: usize) -> Self {
        let n = ((0..inner.len()).rev())
            .map(|i| (i, inner.get(i)))
            .scan(0, |n, (i, s)| {
                *n += s?.len();
                Some((i, *n))
            })
            .find_map(|(idx, len)| if len >= seq_len { Some(idx) } else { None })
            .unwrap_or(0);
        let this = Self { inner, n, seq_len };
        this
    }
}

impl Dataset<Vec<u32>> for SeqLenWrapper {
    fn get(&self, idx: usize) -> Option<Vec<u32>> {
        let mut res = Vec::with_capacity(self.seq_len);
        for i in idx..self.inner.len() {
            res.extend(self.inner.get(i)?);
            if res.len() >= self.seq_len {
                res.truncate(self.seq_len);
                break;
            }
        }
        if res.len() < self.seq_len { None } else { Some(res) }
    }

    fn len(&self) -> usize {
        self.n
    }
}

#[derive(Debug)]
pub struct RandConcat {
    inner: ChatFile,
}

impl RandConcat {
    pub fn new(inner: ChatFile) -> Self {
        Self { inner }
    }

    fn sample_len(&self) -> usize {
        let mut rng = rand::rng();
        let u1: f64 = rng.random_range(f64::EPSILON..1.0);
        let u2: f64 = rng.random();
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let normal = 8.0 + z0 * 2.0_f64.sqrt();
        let result = normal.exp().min(16.0).max(1.0);
        result as usize
    }
}

impl Dataset<Vec<u32>> for RandConcat {
    fn get(&self, idx: usize) -> Option<Vec<u32>> {
        let d = self.sample_len();
        self.inner.get_range(idx..idx + d)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use rand::RngExt;
    use tokenizers::Tokenizer;

    use super::*;
    use crate::read_messages::MsgBundle;

    #[test]
    fn test_dataset_loading() -> anyhow::Result<()> {
        let msgs: Vec<String> = MsgBundle::from_file("data/result.json")?.iter()?.collect();
        let dataset = ChatFile::new("data/whole_dataset.bin")?;
        let tokenizer =
            Tokenizer::from_file("data/tokenizer.json").map_err(anyhow::Error::from_boxed)?;

        assert_eq!(dataset.len(), msgs.len(), "dataset length mismatch");

        let mut rng = rand::rng();
        for _ in 0..40.min(msgs.len()) {
            let idx = rng.random_range(0..msgs.len());
            let msg = &msgs[idx];
            let expected: Vec<u32> = tokenizer
                .encode(msg.as_str(), true)
                .map_err(anyhow::Error::from_boxed)?
                .get_ids()
                .to_vec();
            let actual = dataset.get(idx).context("get from dataset")?;
            assert_eq!(actual, expected, "mismatch at index {}", idx);
        }

        Ok(())
    }
}
