use crate::protocol::ProtocolTypeInfo;
use crate::protocol::Int;
use crate::protocol::Struct;
use std::collections::HashMap;
use std::cmp::min;

struct BitPackedBuffer {
    data: Vec<u8>,
    used: usize,
    next: Option<u8>,
    nextbits: usize,
    bigendian: bool,
}

pub struct BitPackedDecoder {
    buffer: BitPackedBuffer,
    typeinfos: Vec<ProtocolTypeInfo>,
}

pub struct VersionedDecoder {
    pub buffer: BitPackedBuffer,
    typeinfos: Vec<ProtocolTypeInfo>,
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

    fn byte_align(&self) {
        self.nextbits = 0;
    }

    fn read_aligned_bytes(&mut self, bytes: usize) -> &[u8] {
        self.byte_align();
        let data = &self.data[self.used..self.used + bytes];
        self.used += bytes;

        if data.len() != bytes as usize {
            panic!("TruncatedError");
        }

        return data;
    }

    fn read_bits(&mut self, bits: u8) -> u8 {
        let mut result = 0;
        let mut resultbits = 0;

        while resultbits != bits {
            if self.nextbits == 0 {
                if self.done() {
                    panic!("TruncatedError");
                }

                self.next = Some(self.data[self.used]);
                self.used +=1;
                self.nextbits = 8;
            }

            let copybits = min((bits - resultbits) as usize, self.nextbits);
            let copy = self.next.unwrap() & ((1 << copybits) - 1);

            if self.bigendian {
                result |= copy << (bits - resultbits - copybits as u8);
            } else {
                result |= copy << resultbits;
            }
            self.next = Some(self.next.unwrap() >> copybits);
            self.nextbits -= copybits;
            resultbits += copybits as u8;
        }

        result
    }

    fn read_unaligned_bytes(&self, bytes: u8) -> &str {
        let read_bytes = String::new();
        for i in 0..bytes {
            read_bytes.push_str(&self.read_bits(8).to_string());
        }

        &read_bytes
    }
}

enum DecoderResult {
    Value(i64),
    Data(Vec<u8>),
    Pair((u8, i8)),
    Bool(bool),
    Null,
}

pub trait Decoder {
    // instance,
    // byte_align,
    // done,
    // used_bits,

    fn instance(&self, typeinfos: Vec<ProtocolTypeInfo>, typeid: u8) -> DecoderResult {
        if typeid as usize >= typeinfos.len() {
            panic!("CorruptedError");
        }

        let typeinfo = typeinfos[typeid as usize];

        match typeinfo {
            ProtocolTypeInfo::Int(bounds) => self._int(bounds),
            ProtocolTypeInfo::Blob(bounds) => self._blob(bounds),
            ProtocolTypeInfo::Bool => self._bool(),
            // ProtocolTypeInfo::Array(bounds, typeid) => self._array(bounds, typeid),
            // ProtocolTypeInfo::Null => DecoderResult::Null,
            // ProtocolTypeInfo::BitArray(bounds) => self._bitarray(bounds),
            // ProtocolTypeInfo::Optional(typeid) => self._optional(typeid),
            // ProtocolTypeInfo::FourCC => self._fourcc(),
            // ProtocolTypeInfo::Choice(bounds, fields) => self._choice(bounds, fields),
            // ProtocolTypeInfo::Struct(fields) => self._struct(fields),
            _other => panic!("Unknown typeinfo {:?}", _other),
        }
    }

    fn byte_align(buffer: BitPackedBuffer) {
        buffer.byte_align()
    }

    fn done(buffer: BitPackedBuffer) -> bool {
        buffer.done()
    }

    fn used_bits(buffer: BitPackedBuffer) -> usize {
        buffer.used_bits()
    }

    fn _int(&self, bounds: Int) -> DecoderResult;

    fn _blob(&self, bounds: Int) -> DecoderResult;

    fn _bool(&self) -> DecoderResult;

    // fn _array(&self, bounds: Int, typeid: u8) -> DecoderResult;

    // fn _bitarray(&self, bounds: Int) -> DecoderResult;

    // fn _optional(&self, typeid: u8) -> DecoderResult;

    // fn _fourcc(&self) -> DecoderResult;

    // fn _choice(&self, bounds: Int, fields: HashMap<u8, (String, u8)>) -> DecoderResult;

    // fn _struct(&self, fields: Vec<Struct>) -> DecoderResult;
}

// impl Decoder for BitPackedDecoder {}

impl BitPackedDecoder {
    // fn _int(&self, bounds: Int) -> DecoderResult {
    //     DecoderResult::Value(bounds.0 + self.buffer.read_bits(bounds.1) as i64)
    // }
    // _array,
    // _bitarray,
    // _blob,
    // _bool,
    // _choice,
    // _fourcc,
    // _int,
    // _optional,
    // _real32,
    // _real64,
    // _struct,
    // _null,
}

impl VersionedDecoder {
    pub fn new(contents: Vec<u8> ,typeinfos: Vec<ProtocolTypeInfo>) -> VersionedDecoder {
        let buffer = BitPackedBuffer::new(contents);

        VersionedDecoder {
            buffer,
            typeinfos,
        }
    }

    fn expect_skip(&self, expected: u8) {
        if self.buffer.read_bits(8) != expected {
            panic!("CorruptedError");
        }
    }

    fn _vint(&self) -> i64 {
        let buf = self.buffer.read_bits(8) as i64;
        let negative = buf & 1;
        let mut result: i64 = (buf >> 1) & 0x3f;
        let bits = 6;

        while (buf & 0x80) != 0 {
            buf = self.buffer.read_bits(8) as i64;
            result |= (buf & 0x7f) << bits;
            bits += 7;
        }

        if negative != 0 {
            result = -result;
        } else {
            result;
        }

        result
    }
}

impl Decoder for VersionedDecoder {
    fn _int(&self, bounds: Int) -> DecoderResult {
        self.expect_skip(9);
        DecoderResult::Value(self._vint())
    }

    fn _blob(&self, bounds: Int) -> DecoderResult {
        self.expect_skip(2);
        let length = self._vint();
        DecoderResult::Data(self.buffer.read_aligned_bytes(length as usize).to_vec())
    }

    fn _bool(&self) -> DecoderResult {
        self.expect_skip(6);
        DecoderResult::Bool(self.buffer.read_bits(8) != 0)
    }

    // fn _array(&self, bounds: Int, typeid: u8) -> DecoderResult {
    //     self.expect_skip(0);
    //     let length = self._vint();

    //     let mut array = vec![];
    //     for i in 0..length {
    //         array.push(self.instance(self.typeinfos, typeid));
    //     }

    //     DecoderResult::Data(array)
    // }

    // fn _bitarray(&self, bounds: Int) -> DecoderResult {
    //     self.expect_skip(1);
    //     let length = self._vint();
    //     DecoderResult::Pair((length, self.buffer.read_aligned_bytes((length + 7) / 8)))
    // }

    // fn _optional(&self, typeid: u8) -> DecoderResult;

    // fn _fourcc(&self) -> DecoderResult;

    // fn _choice(&self, bounds: Int, fields: HashMap<u8, (String, u8)>) -> DecoderResult;

    // fn _struct(&self, fields: Vec<Struct>) -> DecoderResult;
}
