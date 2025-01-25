pub trait Pattern {
    type Output<'a> = ();
    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)>;
}

pub struct EofPat;
impl Pattern for EofPat {
    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
        input.is_empty().then_some(("", ()))
    }
}

impl Pattern for &str {
    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
        input
            .starts_with(&*self)
            .then(|| (input.split_at(self.len()).1, ()))
    }
}
impl Pattern for char {
    type Output<'a> = char;
    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
        let mut it = input.chars();
        (it.next()? == *self).then(|| (it.as_str(), *self))
    }
}

pub struct OfPred<T: FnMut(char) -> bool>(T);
impl<T: FnMut(char) -> bool> Pattern for OfPred<T> {
    type Output<'a> = &'a str;
    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
        let i = input
            .char_indices()
            .find(|(_, c)| !(self.0)(*c))
            .map_or_else(|| input.len(), |(i, _)| i);
        let (find, rest) = input.split_at(i);
        Some((rest, find))
    }
}
pub fn of_pred<F: FnMut(char) -> bool>(f: F) -> OfPred<F> {
    OfPred(f)
}

pub struct Maybe<P: Pattern>(P);
impl<P: Pattern> Pattern for Maybe<P> {
    type Output<'a> = Option<P::Output<'a>>;

    fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
        Some(
            self.0
                .check_pattern(input)
                .map_or((input, None), |(cdr, res)| (cdr, Some(res))),
        )
    }
}
pub fn maybe<P: Pattern>(p: P) -> Maybe<P> {
    Maybe(p)
}

macro_rules! impl_tuple {
    (@ $($ts:ident)*) => {
        impl<$($ts: Pattern,)*> Pattern for ($($ts,)*) {
            type Output<'a> = ($($ts::Output<'a>,)*);
            fn check_pattern<'a>(&mut self, input: &'a str) -> Option<(&'a str, Self::Output<'a>)> {
                $(#[allow(non_snake_case)]
                let (input, $ts) = self.${index()}.check_pattern(input)?;)*
                Some((input, ($($ts,)*)))
            }
        }
    };
    ([$($xs:ident)*]) => { impl_tuple!{@ $($xs)*} };
    ([$($xs:ident)*] $y:ident $($ys:ident)*) => {
        impl_tuple!{@ $($xs)*}
        impl_tuple!{[$($xs)* $y] $($ys)*}
    };
}
impl_tuple! {[T1] T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15 T16 T17 T18 T19 T20}
