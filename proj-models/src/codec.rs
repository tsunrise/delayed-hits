use std::io::{Read, Write};

/// A trait for encoding and decoding data.
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
    use std::io::{Read, Write};

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

    impl Codec for TestStruct {
        type Deserialized = Self;

        fn size_in_bytes(&self) -> usize {
            self.a.size_in_bytes() + self.b.size_in_bytes()
        }

        fn to_bytes<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
            self.a.to_bytes(writer.by_ref())?;
            self.b.to_bytes(writer.by_ref())
        }

        fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
            let a = u64::from_bytes(&mut reader)?;
            let b = u32::from_bytes(&mut reader)?;
            Ok(Self { a, b })
        }
    }

    impl Codec for TestStruct2 {
        type Deserialized = Self;

        fn size_in_bytes(&self) -> usize {
            self.a.size_in_bytes() + self.b.size_in_bytes()
        }

        fn to_bytes<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
            self.a.to_bytes(writer.by_ref())?;
            self.b.to_bytes(writer.by_ref())
        }

        fn from_bytes<R: Read>(mut reader: R) -> std::io::Result<Self::Deserialized> {
            let a = u64::from_bytes(&mut reader)?;
            let b = u16::from_bytes(&mut reader)?;
            Ok(Self { a, b })
        }
    }

    #[test]
    fn test_repeat_read_write() {
        let lst = (0..100)
            .map(|i| TestStruct { a: i, b: i as u32 })
            .collect::<Vec<_>>();
        let lst2 = (118..192)
            .map(|i| TestStruct2 { a: i, b: i as u16 })
            .collect::<Vec<_>>();
        let mut buf = Vec::new();
        TestStruct::repeat_write_with_known_len(&mut buf, &lst, lst.len()).unwrap();
        TestStruct2::repeat_write_till_end(&mut buf, &lst2).unwrap();
        let mut reader = std::io::Cursor::new(buf);
        let read_lst1 = TestStruct::repeat_read_with_known_len(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let read_lst2 = TestStruct2::repeat_read_till_end(reader)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(lst, read_lst1);
        assert_eq!(lst2, read_lst2);
    }
}