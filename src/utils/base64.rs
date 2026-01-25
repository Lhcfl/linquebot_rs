//! 用来练手的 base64 encode/decoder (
use std::{fmt, slice::Iter};

#[derive(Debug, PartialEq)]
pub enum Base64Error {
    IllegalBase64Symbol(char),
    NotLongEnough(String),
}

impl fmt::Display for Base64Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IllegalBase64Symbol(c) => write!(f, "存在不应该出现在 Base64 中的字符: {c}"),
            NotLongEnough(str) => write!(f, "输入的长度不能被4整除。已解码的内容: {str}"),
        }
    }
}

use Base64Error::*;

fn u2c(x: u8) -> char {
    (match x {
        0..26 => b'A' + x,
        26..52 => b'a' + x - 26,
        52..62 => b'0' + x - 52,
        62 => b'+',
        63 => b'/',
        _ => unreachable!(),
    }) as char
}

fn c2u(x: u8) -> Result<u8, Base64Error> {
    Ok(match x as char {
        'A'..='Z' => x - b'A',
        'a'..='z' => x - b'a' + 26,
        '0'..='9' => x - b'0' + 52,
        '+' => 62,
        '/' => 63,
        '=' => 0,
        x => return Err(IllegalBase64Symbol(x)),
    })
}

pub fn encode_bytes(mut bytes: Iter<u8>) -> String {
    let mut res = String::new();
    let mut padding = 0;
    let mut done = false;

    while !done {
        let [x, y, z] = bytes.next_chunk().unwrap_or_else(|mut last| {
            let mut push = || {
                last.next().unwrap_or_else(|| {
                    padding += 1;
                    &0
                })
            };
            done = true;
            [push(), push(), push()]
        });
        res.push(u2c((x & 0b11111100) >> 2));
        res.push(u2c(((x & 0b00000011) << 4) | ((y & 0b11110000) >> 4)));
        res.push(u2c(((y & 0b00001111) << 2) | ((z & 0b11000000) >> 6)));
        res.push(u2c(z & 0b00111111));
    }
    if padding == 3 {
        for _ in 0..4 {
            res.pop();
        }
    } else {
        let tail = res.len() - padding;
        unsafe { res.as_bytes_mut()[tail..].fill(b'=') };
    }
    res
}

pub fn encode(str: &str) -> String {
    encode_bytes(str.as_bytes().iter())
}

pub fn decode_bytes(str: &str) -> Result<Vec<u8>, Base64Error> {
    let mut resu8 = Vec::<u8>::new();
    for [a, b, c, d] in str.bytes().array_chunks() {
        resu8.push((c2u(a)? << 2) | (c2u(b)? >> 4));
        resu8.push(((c2u(b)? & 0b00001111) << 4) | (c2u(c)? >> 2));
        resu8.push(((c2u(c)? & 0b00000011) << 6) | c2u(d)?);

        resu8.pop_if(|_| c == b'=');
        resu8.pop_if(|_| d == b'=');
    }
    if str.len().is_multiple_of(4) {
        Ok(resu8)
    } else {
        Err(NotLongEnough(String::from_utf8_lossy(&resu8).into_owned()))
    }
}

pub fn decode(str: &str) -> Result<String, Base64Error> {
    Ok(String::from_utf8_lossy(&decode_bytes(str)?).into_owned())
}

#[cfg(test)]
mod base64_test {
    use rand::random;

    use super::*;

    #[test]
    fn encode_test() {
        assert_eq!(encode(""), "");
        assert_eq!(encode("1"), "MQ==");
        assert_eq!(encode("12"), "MTI=");
        assert_eq!(encode("123"), "MTIz");
        assert_eq!(encode("1234"), "MTIzNA==");
        assert_eq!(encode("中文测试"), "5Lit5paH5rWL6K+V");
        assert_eq!(encode("ああづいしオア"), "44GC44GC44Gl44GE44GX44Kq44Ki");
    }

    #[test]
    fn decode_test() {
        assert_eq!("", decode("").unwrap());
        assert_eq!("1", decode("MQ==").unwrap());
        assert_eq!("12", decode("MTI=").unwrap());
        assert_eq!("123", decode("MTIz").unwrap());
        assert_eq!("1234", decode("MTIzNA==").unwrap());
        assert_eq!("中文测试", decode("5Lit5paH5rWL6K+V").unwrap());
        assert_eq!(
            "ああづいしオア",
            decode("44GC44GC44Gl44GE44GX44Kq44Ki").unwrap()
        );
        assert_eq!(
            Base64Error::IllegalBase64Symbol(';'),
            decode(";;;;;").unwrap_err()
        );
    }

    #[test]
    fn encode_should_eq_decode_test() {
        for _ in 0..10000 {
            let str_len: u8 = random();
            let mut u8str = Vec::<u8>::new();
            for _ in 1..str_len {
                u8str.push(random());
            }
            let str = String::from_utf8_lossy(&u8str).into_owned();
            assert_eq!(str, decode(&encode(&str)).unwrap());
        }
    }

    #[test]
    fn encode_vec_should_eq_decode_vec_test() {
        for _ in 0..10000 {
            let str_len: u8 = random();
            let mut u8vec = Vec::<u8>::new();
            for _ in 1..str_len {
                u8vec.push(random());
            }
            assert_eq!(u8vec, decode_bytes(&encode_bytes(u8vec.iter())).unwrap());
        }
    }
}
