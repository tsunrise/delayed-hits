use std::io::{Read, Write};

/// A trait for encoding and decoding a fixed-size data.
pub trait Codec {
    type Deserialized: Sized;
    fn size_in_bytes(&self) -> usize;
    fn to_bytes<W: Write>(&self, writer: W) -> std::io::Result<()>;
    fn from_bytes<R: Read>(reader: R) -> std::io::Result<Self::Deserialized>;
    fn repeat_write_till_end<'a, W, I>(mut writer: W, iter: I) -> std::io::Result<()>
    where
        W: Write,
        I: IntoIterator<Item = &'a Self>,
        Self: Sized + 'a,
    {
        for item in iter {
            item.to_bytes(writer.by_ref())?;
        }
        Ok(())
    }
    fn repeat_read_till_end<R: Read>(reader: R) -> ReadTillEndIterator<Self, R>
    where
        Self: Sized,
    {
        ReadTillEndIterator {
            reader,
            _phantom: std::marker::PhantomData,
        }
    }

    fn repeat_write_with_known_len<'a, W, I>(
        mut writer: W,
        iter: I,
        len: usize,
    ) -> std::io::Result<()>
    where
        W: Write,
        I: IntoIterator<Item = &'a Self>,
        Self: Sized + 'a,
    {
        let len = len as u64;
        len.to_bytes(&mut writer)?;
        Self::repeat_write_till_end(writer, iter)
    }

    fn repeat_read_with_known_len<R: Read>(mut reader: R) -> ReadWithKnownLenIterator<Self, R>
    where
        Self: Sized,
    {
        let len = u64::from_bytes(&mut reader).unwrap() as usize;
        ReadWithKnownLenIterator {
            reader,
            len,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[macro_export]
macro_rules! impl_codec {
    ($struct:ident, $($field:ident, $field_type:ty),+) => {
        impl $crate::codec::Codec for $struct {
            type Deserialized = Self;

            fn size_in_bytes(&self) -> usize {
                $(self.$field.size_in_bytes() +)* 0
            }

            fn to_bytes<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
                $(self.$field.to_bytes(&mut writer)?;)*
                Ok(())
            }

            fn from_bytes<R: std::io::Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
                Ok(Self {
                    $($field: <$field_type>::from_bytes(&mut reader)?),*
                })
            }
        }
    };
}

macro_rules! impl_codec_for_primitive {
    ($t:ty) => {
        impl Codec for $t {
            type Deserialized = $t;

            fn size_in_bytes(&self) -> usize {
                std::mem::size_of::<$t>()
            }

            fn to_bytes<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
                writer.write_all(&self.to_le_bytes())
            }

            fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
                let mut buf = [0; std::mem::size_of::<$t>()];
                reader.read_exact(&mut buf)?;
                Ok(<$t>::from_le_bytes(buf))
            }
        }
    };
}

impl_codec_for_primitive!(u8);
impl_codec_for_primitive!(u16);
impl_codec_for_primitive!(u32);
impl_codec_for_primitive!(u64);

impl<T: Codec> Codec for [T] {
    type Deserialized = Vec<T::Deserialized>;

    fn size_in_bytes(&self) -> usize {
        self.iter().map(|v| v.size_in_bytes()).sum::<usize>() + std::mem::size_of::<u64>()
    }

    fn to_bytes<W: Write>(&self, writer: W) -> std::io::Result<()> {
        T::repeat_write_with_known_len(writer, self, self.len())
    }

    fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
        let mut len = u64::from_bytes(&mut reader)? as usize;
        let mut vec = Vec::with_capacity(len);
        while len > 0 {
            vec.push(T::from_bytes(&mut reader)?);
            len -= 1;
        }
        Ok(vec)
    }
}

impl<T: Codec> Codec for Vec<T> {
    type Deserialized = Vec<T::Deserialized>;

    fn size_in_bytes(&self) -> usize {
        self.as_slice().size_in_bytes()
    }

    fn to_bytes<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        self.as_slice().to_bytes(&mut writer)
    }

    fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
        <[T]>::from_bytes(&mut reader)
    }
}

pub struct ReadTillEndIterator<T: Codec, R: Read> {
    reader: R,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Codec, R: Read> Iterator for ReadTillEndIterator<T, R> {
    type Item = std::io::Result<T::Deserialized>;

    fn next(&mut self) -> Option<Self::Item> {
        match T::from_bytes(&mut self.reader) {
            Ok(v) => Some(Ok(v)),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    None
                } else {
                    Some(Err(e))
                }
            }
        }
    }
}

pub struct ReadWithKnownLenIterator<T: Codec, R: Read> {
    reader: R,
    len: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Codec, R: Read> Iterator for ReadWithKnownLenIterator<T, R> {
    type Item = std::io::Result<T::Deserialized>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        match T::from_bytes(&mut self.reader) {
            Ok(v) => {
                self.len -= 1;
                Some(Ok(v))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Codec;

    #[derive(Debug, Eq, PartialEq)]
    struct TestStruct {
        a: u64,
        b: u32,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TestStruct2 {
        a: u64,
        b: u16,
    }
    #[derive(Debug, Eq, PartialEq)]
    struct TestStruct3 {
        a: u64,
        b: u64,
        c: u8,
    }

    impl_codec!(TestStruct, a, u64, b, u32);
    impl_codec!(TestStruct2, a, u64, b, u16);
    impl_codec!(TestStruct3, a, u64, b, u64, c, u8);

    #[test]
    fn test_repeat_read_write() {
        let lst = (0..100)
            .map(|i| TestStruct { a: i, b: i as u32 })
            .collect::<Vec<_>>();
        let lst2 = (118..192)
            .map(|i| TestStruct2 { a: i, b: i as u16 })
            .collect::<Vec<_>>();
        let lst3 = (0..100)
            .map(|i| TestStruct3 {
                a: i,
                b: i as u64,
                c: i as u8,
            })
            .collect::<Vec<_>>();
        let mut buf = Vec::new();
        TestStruct::repeat_write_with_known_len(&mut buf, &lst, lst.len()).unwrap();
        lst2.to_bytes(&mut buf).unwrap();
        TestStruct3::repeat_write_till_end(&mut buf, &lst3).unwrap();

        let mut reader = std::io::Cursor::new(buf);
        let lst1_actual = TestStruct::repeat_read_with_known_len(&mut reader)
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();
        let lst2_actual = Vec::<TestStruct2>::from_bytes(&mut reader).unwrap();
        let lst3_actual = TestStruct3::repeat_read_till_end(&mut reader)
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(lst, lst1_actual);
        assert_eq!(lst2, lst2_actual);
        assert_eq!(lst3, lst3_actual);
    }
}
