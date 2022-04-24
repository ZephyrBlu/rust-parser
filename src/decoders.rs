use crate::protocol::ProtocolTypeInfo;
use crate::protocol::Int;
use crate::protocol::Struct;
use std::collections::HashMap;
use std::cmp::min;
use std::str;

pub struct BitPackedBuffer {
    data: Vec<u8>,
    used: usize,
    next: Option<u8>,
    nextbits: usize,
    bigendian: bool,
}

pub struct BitPackedDecoder<'a> {
    pub buffer: BitPackedBuffer,
    typeinfos: &'a Vec<ProtocolTypeInfo<'a>>,
}

pub struct VersionedDecoder<'a> {
    pub buffer: BitPackedBuffer,
    typeinfos: &'a Vec<ProtocolTypeInfo<'a>>,
}

impl BitPackedBuffer {
    fn new(contents: Vec<u8>) -> BitPackedBuffer {
        BitPackedBuffer {
            data: contents,
            used: 0,
            next: None,
            nextbits: 0,
            bigendian: true,
        }
    }

    fn done(&self) -> bool {
        self.used >= self.data.len()
    }


    fn used_bits(&self) -> usize {
        (self.used * 8) - self.nextbits
    }

    fn byte_align(&mut self) {
        self.nextbits = 0;
    }

    fn read_aligned_bytes(&mut self, bytes: usize) -> &[u8] {
        self.byte_align();
        let data = &self.data[self.used..self.used + bytes];
        self.used += bytes;

        if data.len() != bytes as usize {
            panic!("TruncatedError");
        }

        data
    }

    fn read_bits(&mut self, bits: u8) -> u128 {
        // usually much smaller than u128, but can be in rare cases
        let mut result: u128 = 0;
        let mut resultbits: u8 = 0;

        while resultbits != bits {
            if self.nextbits == 0 {
                if self.done() {
                    panic!("TruncatedError");
                }

                self.next = Some(self.data[self.used]);
                self.used +=1;
                self.nextbits = 8;
            }

            let copybits: u8 = min((bits - resultbits) as usize, self.nextbits) as u8;
            let shifted_copybits: u8 = ((1 << copybits) - 1) as u8;
            let copy: u128 = (self.next.unwrap() & shifted_copybits) as u128;

            if self.bigendian {
                result |= copy << (bits - resultbits - copybits);
            } else {
                result |= copy << resultbits;
            }
            let shifted_next: u8 = (self.next.unwrap() as u16 >> copybits) as u8;
            self.next = Some(shifted_next);
            self.nextbits -= copybits as usize;
            resultbits += copybits as u8;
        }

        result
    }

    fn read_unaligned_bytes(&mut self, bytes: u8) -> String {
        let mut read_bytes = String::new();
        for i in 0..bytes {
            read_bytes.push_str(&self.read_bits(8).to_string());
        }

        read_bytes
    }
}

#[derive(Debug)]
pub enum StructValue<'a> {
    String(String),
    Int(u8),
    Struct(HashMap<&'a str, StructValue<'a>>),
    Data(Vec<u32>),
    Bool(bool),
    Pair((u8, i8)),
    Null,
}

#[derive(Debug)]
pub enum DecoderResult<'a> {
    Value(i64),
    BlobData(Vec<u8>),
    ArrayData(Vec<u32>),
    Pair((u8, i8)),
    Bool(bool),
    Struct(HashMap<&'a str, StructValue<'a>>),
    Null,
}

pub trait Decoder<'decode> {
    fn instance<'a>(&'a mut self, typeinfos: &'a Vec<ProtocolTypeInfo<'decode>>, typeid: u8) -> DecoderResult<'decode> {
        if typeid as usize >= typeinfos.len() {
            panic!("CorruptedError");
        }

        let typeinfo = &typeinfos[typeid as usize];
        // println!("current typeinfo {:?} {:?}", typeinfo, typeid);

        match typeinfo {
            ProtocolTypeInfo::Int(bounds) => self._int(*bounds),
            ProtocolTypeInfo::Blob(bounds) => self._blob(*bounds),
            ProtocolTypeInfo::Bool => self._bool(),
            ProtocolTypeInfo::Array(bounds, typeid) => self._array(*bounds, *typeid),
            ProtocolTypeInfo::Null => DecoderResult::Null,
            ProtocolTypeInfo::BitArray(bounds) => self._bitarray(*bounds),
            ProtocolTypeInfo::Optional(typeid) => self._optional(*typeid),
            // ProtocolTypeInfo::FourCC => self._fourcc(),
            ProtocolTypeInfo::Choice(bounds, fields) => self._choice(*bounds, fields),
            ProtocolTypeInfo::Struct(fields) => self._struct(fields),
            _other => panic!("Unknown typeinfo {:?}", _other),
        }
    }

    fn byte_align(buffer: &mut BitPackedBuffer) {
        buffer.byte_align()
    }

    fn done(buffer: &BitPackedBuffer) -> bool {
        buffer.done()
    }

    fn used_bits(buffer: &BitPackedBuffer) -> usize {
        buffer.used_bits()
    }

    fn _int(&mut self, bounds: Int) -> DecoderResult<'decode>;

    fn _blob(&mut self, bounds: Int) -> DecoderResult<'decode>;

    fn _bool(&mut self) -> DecoderResult<'decode>;

    fn _array(&mut self, bounds: Int, typeid: u8) -> DecoderResult<'decode>;

    fn _bitarray(&mut self, bounds: Int) -> DecoderResult<'decode>;

    fn _optional(&mut self, typeid: u8) -> DecoderResult<'decode>;

    // fn _fourcc(&self) -> DecoderResult;

    fn _choice(&mut self, bounds: Int, fields: &HashMap<i64, (String, u8)>) -> DecoderResult<'decode>;

    fn _struct<'a>(&'a mut self, fields: &'a Vec<Struct<'decode>>) -> DecoderResult<'decode>;
}

impl BitPackedDecoder<'_> {
    pub fn new<'a>(contents: Vec<u8>, typeinfos: &'a Vec<ProtocolTypeInfo<'static>>) -> BitPackedDecoder<'a> {
        let buffer = BitPackedBuffer::new(contents);

        BitPackedDecoder {
            buffer,
            typeinfos,
        }
    }
}

impl<'decode> Decoder<'decode> for BitPackedDecoder<'decode> {
    fn _int(&mut self, bounds: Int) -> DecoderResult<'decode> {
        let read = self.buffer.read_bits(bounds.1);
        DecoderResult::Value(bounds.0 + read as i64)
    }

    fn _blob(&mut self, bounds: Int) -> DecoderResult<'decode> {
        match self._int(bounds) {
            DecoderResult::Value(value) => DecoderResult::BlobData(self.buffer.read_aligned_bytes(value as usize).to_vec()),
            _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
        }
    }

    fn _bool(&mut self) -> DecoderResult<'decode> {
        match self._int(Int(0, 1)) {
            DecoderResult::Value(value) => DecoderResult::Bool(value != 0),
            _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
        }
    }

    fn _array(&mut self, bounds: Int, typeid: u8) -> DecoderResult<'decode> {
        match self._int(bounds) {
            DecoderResult::Value(value) => {
                let mut result = vec![];
                for i in 0..value {
                    let data = match self.instance(self.typeinfos, typeid) {
                        DecoderResult::Value(_value) => _value,
                        _other => {
                            // println!("instance didn't return DecoderResult::Value {:?}", _other);
                            0
                        },
                    };
                    result.push(data as u32);
                }

                DecoderResult::ArrayData(result)
                
            },
            _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
        }
    }

    fn _bitarray(&mut self, bounds: Int) -> DecoderResult<'decode> {
        match self._int(bounds) {
            DecoderResult::Value(value) => {
                let bytes = self.buffer.read_bits(value as u8);
                DecoderResult::Pair((0, 0))
            },
            _other => panic!("instance didn't return DecoderResult::Value {:?}", _other),
        }
    }

    fn _optional(&mut self, typeid: u8) -> DecoderResult<'decode> {
        match self._bool() {
            DecoderResult::Bool(value) => {
                if value {
                    self.instance(self.typeinfos, typeid)
                } else {
                    DecoderResult::Null
                }
            },
            _other => panic!("_bool didn't return DecoderResult::Bool {:?}", _other),
        }
    }

    // fn _fourcc(&self) -> DecoderResult;

    fn _choice(&mut self, bounds: Int, fields: &HashMap<i64, (String, u8)>) -> DecoderResult<'decode> {
        let tag = match self._int(bounds) {
            DecoderResult::Value(value) => value,
            _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
        };

        if !fields.contains_key(&tag) {
            panic!("CorruptedError");
        }
        let field = &fields[&tag];
        let choice_res = self.instance(self.typeinfos, field.1);
        // println!("_choice instance returned {:?} {:?}", field.0, choice_res);
        DecoderResult::Pair((0, 0))
    }

    fn _struct<'a>(&mut self, fields: &'a Vec<Struct<'decode>>) -> DecoderResult<'decode> {
        let mut result = HashMap::<&str, StructValue>::new();
        for field in fields {
            // appears that this isn't needed since field is never parent
            // match fields.into_iter().find(|f| f.2 as i64 == tag) {
            //     Some(field) => {
            //         if field.0 == "__parent" {
            //             let parent = self.instance(self.typeinfos, field.1);
            //         } else {
                        // let field_value = match self.instance(self.typeinfos, field.1) {
                        //     DecoderResult::Value(value) => value,
                        //     _other => panic!("field.1 is not a value: {:?}", field),
                        // };
                        // result.insert(field.0.as_str(), field_value as u8);
            //         }
            //     },
            //     None => self._skip_instance(),
            // };

            // field always seems to exist?
            let field_value = match self.instance(self.typeinfos, field.1) {
                DecoderResult::Value(value) => StructValue::Int(value as u8),
                DecoderResult::BlobData(values) => StructValue::String(String::from_utf8(values).unwrap()),
                DecoderResult::ArrayData(values) => StructValue::Data(values),
                DecoderResult::Struct(value) => StructValue::Struct(value),
                DecoderResult::Bool(value) => StructValue::Bool(value),
                DecoderResult::Pair(value) => StructValue::Pair(value),
                DecoderResult::Null => StructValue::Null,
            };
            // println!("field values {:?} {:?}", field, field_value);
            result.insert(field.0, field_value);
        }

        DecoderResult::Struct(result)
    }
}

impl VersionedDecoder<'_> {
    pub fn new<'a>(contents: Vec<u8>, typeinfos: &'a Vec<ProtocolTypeInfo<'static>>) -> VersionedDecoder<'a> {
        let buffer = BitPackedBuffer::new(contents);

        VersionedDecoder {
            buffer,
            typeinfos,
        }
    }

    fn expect_skip(&mut self, expected: u8) {
        let bits_read = self.buffer.read_bits(8);
        if bits_read as u8 != expected {
            panic!("CorruptedError");
        }
    }

    fn _vint(&mut self) -> i64 {
        let mut buf = self.buffer.read_bits(8) as i64;
        let negative = buf & 1;
        let mut result: i64 = (buf >> 1) & 0x3f;
        let mut bits = 6;

        while (buf & 0x80) != 0 {
            buf = self.buffer.read_bits(8) as i64;
            result |= (buf & 0x7f) << bits;
            bits += 7;
        }

        if negative != 0 {
            -result
        } else {
            result
        }
    }

    fn _skip_instance(&mut self) {
        let skip = self.buffer.read_bits(8);
        if skip == 0 {  // array
            let length = self._vint();
            for i in 0..length {
                self._skip_instance();
            }
        } else if skip == 1 {  // bitblob
            let length = self._vint();
            self.buffer.read_aligned_bytes(((length + 7) / 8) as usize);
        } else if skip == 2 {  // blob
            let length = self._vint();
            self.buffer.read_aligned_bytes(length as usize);
        } else if skip == 3 {  // choice
            let tag = self._vint();
            self._skip_instance();
        } else if skip == 4 {  // optional
            let exists = self.buffer.read_bits(8) != 0;
            if exists {
                self._skip_instance();
            }
        } else if skip == 5 {  // struct
            let length = self._vint();
            for i in 0..length {
                let tag = self._vint();
                self._skip_instance();
            }
        } else if skip == 6 {  // u8
            self.buffer.read_aligned_bytes(1);
        } else if skip == 7 {  // u32
            self.buffer.read_aligned_bytes(4);
        } else if skip == 8 {  // u64
            self.buffer.read_aligned_bytes(8);
        } else if skip == 9 {  // vint
            self._vint();
        }
    }
}

impl<'decode> Decoder<'decode> for VersionedDecoder<'decode> {
    fn _int(&mut self, bounds: Int) -> DecoderResult<'decode> {
        self.expect_skip(9);
        DecoderResult::Value(self._vint())
    }

    fn _blob(&mut self, bounds: Int) -> DecoderResult<'decode> {
        self.expect_skip(2);
        let length = self._vint();
        DecoderResult::BlobData(self.buffer.read_aligned_bytes(length as usize).to_vec())
    }

    fn _bool(&mut self) -> DecoderResult<'decode> {
        self.expect_skip(6);
        DecoderResult::Bool(self.buffer.read_bits(8) != 0)
    }

    fn _array(&mut self, bounds: Int, typeid: u8) -> DecoderResult<'decode> {
        self.expect_skip(0);
        let length = self._vint();

        let mut array = vec![];
        for i in 0..length {
            let data = match self.instance(self.typeinfos, typeid) {
                DecoderResult::Value(value) => value,
                _other => panic!("instance didn't return DecoderResult::Value {:?}", _other),
            };
            array.push(data as u32);
        }

        DecoderResult::ArrayData(array)
    }

    fn _bitarray(&mut self, bounds: Int) -> DecoderResult<'decode> {
        self.expect_skip(1);
        let length = self._vint();
        let bytes = self.buffer.read_aligned_bytes((length as usize + 7) / 8);
        DecoderResult::Pair((0, 0))
    }

    fn _optional(&mut self, typeid: u8) -> DecoderResult<'decode> {
        self.expect_skip(4);
        if self.buffer.read_bits(8) != 0 {
            self.instance(self.typeinfos, typeid)
        } else {
            DecoderResult::Null
        }
    }

    // fn _fourcc(&self) -> DecoderResult;

    fn _choice(&mut self, bounds: Int, fields: &HashMap<i64, (String, u8)>) -> DecoderResult<'decode> {
        self.expect_skip(3);
        let tag = self._vint();
        if !fields.contains_key(&tag) {
            self._skip_instance();
            return DecoderResult::Pair((0, 0))
        }
        let field = &fields[&tag];
        let choice_res = self.instance(self.typeinfos, field.1);
        // println!("_choice instance returned {:?} {:?}", field.0, choice_res);
        DecoderResult::Pair((0, 0))
    }

    fn _struct<'a>(&mut self, fields: &'a Vec<Struct<'decode>>) -> DecoderResult<'decode> {
        self.expect_skip(5);
        let mut result = HashMap::<&str, StructValue>::new();
        let length = self._vint();
        for i in 0..length {
            let tag = self._vint();

            // appears that this isn't needed since field is never parent
            // match fields.into_iter().find(|f| f.2 as i64 == tag) {
            //     Some(field) => {
            //         if field.0 == "__parent" {
            //             let parent = self.instance(self.typeinfos, field.1);
            //         } else {
                        // let field_value = match self.instance(self.typeinfos, field.1) {
                        //     DecoderResult::Value(value) => value,
                        //     _other => panic!("field.1 is not a value: {:?}", field),
                        // };
                        // result.insert(field.0.as_str(), field_value as u8);
            //         }
            //     },
            //     None => self._skip_instance(),
            // };

            // field always seems to exist?
            let field = fields.into_iter().find(|f| f.2 as i64 == tag).unwrap();
            let field_instance = self.instance(self.typeinfos, field.1);
            // println!("field instance {:?} {:?}", field, field_instance);
            let field_value = match field_instance {
                DecoderResult::Value(value) => StructValue::Int(value as u8),
                DecoderResult::BlobData(values) => StructValue::String(String::from_utf8(values).unwrap()),
                DecoderResult::ArrayData(values) => StructValue::Data(values),
                DecoderResult::Struct(value) => StructValue::Struct(value),
                DecoderResult::Null => StructValue::Null,
                _other => panic!("field.1 is not a value or blob: {:?}", field),
            };
            // println!("field values {:?} {:?}", field, field_value);
            result.insert(field.0, field_value);
        }

        DecoderResult::Struct(result)
    }
}
