//! Positive fixture extraction fails loudly; no lint exceptions or ignored errors.
pub trait Required {
    type Output;
    fn required(self) -> Self::Output;
}
impl<T, E: core::fmt::Debug> Required for Result<T, E> {
    type Output = T;
    fn required(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("required fixture/result failed: {error:?}"),
        }
    }
}
impl<T> Required for Option<T> {
    type Output = T;
    fn required(self) -> T {
        match self {
            Some(value) => value,
            None => panic!("required fixture field/result is absent"),
        }
    }
}
