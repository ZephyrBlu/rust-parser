use crate::protocol::Int;
use crate::protocol::ProtocolTypeInfo;
use crate::protocol::Struct;

use std::str;

pub struct BitPackedBuffer {
  data: Vec<u8>,
  data_len: usize,
  used: usize,
  next: u8,
  nextbits: usize,
  bigendian: bool,
}

pub struct BitPackedDecoder<'a> {
  pub buffer: BitPackedBuffer,
  typeinfos: &'a [ProtocolTypeInfo<'a>],
}

pub struct VersionedDecoder<'a> {
  pub buffer: BitPackedBuffer,
  typeinfos: &'a [ProtocolTypeInfo<'a>],
}

impl BitPackedBuffer {
  fn new(contents: Vec<u8>) -> BitPackedBuffer {
    let data_len = contents.len();
    BitPackedBuffer {
      data: contents,
      data_len,
      used: 0,
      next: 0,
      nextbits: 0,
      bigendian: true,
    }
  }

  // fn done(&self) -> bool {
  //   self.used >= self.data_len
  // }

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
        if self.used >= self.data_len {
          panic!("TruncatedError");
        }

        self.next = self.data[self.used];
        self.used += 1;
        self.nextbits = 8;
      }

      // let copybits: u8 = min((bits - resultbits) as usize, self.nextbits) as u8;
      let copybits: u8 = if (bits - resultbits) < self.nextbits as u8 {
        bits - resultbits
      } else {
        self.nextbits as u8
      };
      let shifted_copybits: u8 = ((1 << copybits) - 1) as u8;
      let copy: u128 = (self.next & shifted_copybits) as u128;

      if self.bigendian {
        result |= copy << (bits - resultbits - copybits);
      } else {
        result |= copy << resultbits;
      }
      let shifted_next: u8 = (self.next as u16 >> copybits) as u8;
      self.next = shifted_next;
      self.nextbits -= copybits as usize;
      resultbits += copybits as u8;
    }

    result
  }

  fn read_unaligned_bytes(&mut self, bytes: u8) -> String {
    let mut read_bytes = String::new();
    for _ in 0..bytes {
      read_bytes.push_str(&self.read_bits(8).to_string());
    }

    read_bytes
  }
}

#[derive(Debug, PartialEq, Clone)]
pub enum EventField {
  Gameloop,
  ControlPlayerId,
  PlayerId,
  UnitTypeName,
  UnitTagIndex,
  UnitTagRecycle,
  Stats,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StatsField {
  WorkersActiveCount,
  MineralsCollectionRate,
  VespeneCollectionRate,
  MineralsCurrent,
  VespeneCurrent,
  MineralsLostArmy,
  MineralsLostEconomy,
  MineralsLostTechnology,
  VespeneLostArmy,
  VespeneLostEconomy,
  VespeneLostTechnology,
  MineralsUsedInProgressArmy,
  MineralsUsedCurrentArmy,
  VespeneUsedInProgressArmy,
  VespeneUsedCurrentArmy,
}

pub enum EventType {
  ObjectEvent,
  PlayerStatsEvent,
}

pub type EventEntry =  (String, DecoderResult);

#[derive(Clone, Debug)]
pub enum DecoderResult {
  Name(String),
  Value(i64),
  Blob(String),
  Array(Vec<DecoderResult>),
  DataFragment(u32),
  Pair((i64, i16)),
  Gameloop((String, i64)),
  Bool(bool),
  Struct(Vec<EventEntry>),
  Null,
  Empty,
}

pub trait Decoder {
  fn instance<'a>(
    &'a mut self,
    typeinfos: &[ProtocolTypeInfo],
    typeid: &u8,
    event_allowed: bool,
  ) -> DecoderResult {
    let typeid_size = *typeid as usize;
    if typeid_size >= typeinfos.len() {
      panic!("CorruptedError");
    }

    let typeinfo = &typeinfos[typeid_size];
    // println!("current typeinfo {:?} {:?}", typeinfo, typeid);

    match typeinfo {
      ProtocolTypeInfo::Int(bounds) => self._int(bounds),
      ProtocolTypeInfo::Blob(bounds) => self._blob(bounds),
      ProtocolTypeInfo::Bool => self._bool(),
      ProtocolTypeInfo::Array(bounds, typeid) => self._array(bounds, typeid, event_allowed),
      ProtocolTypeInfo::Null => DecoderResult::Null,
      ProtocolTypeInfo::BitArray(bounds) => self._bitarray(bounds),
      ProtocolTypeInfo::Optional(typeid) => self._optional(typeid, event_allowed),
      ProtocolTypeInfo::FourCC => self._fourcc(),
      ProtocolTypeInfo::Choice(bounds, fields) => self._choice(bounds, fields, event_allowed),
      ProtocolTypeInfo::Struct(fields) => self._struct(fields, event_allowed),
    }
  }

  fn byte_align(buffer: &mut BitPackedBuffer) {
    buffer.byte_align()
  }

  fn done(buffer: &BitPackedBuffer) -> bool {
    // buffer.done()
    buffer.used >= buffer.data_len
  }

  fn used_bits(buffer: &BitPackedBuffer) -> usize {
    buffer.used_bits()
  }

  fn _int(&mut self, bounds: &Int) -> DecoderResult;

  fn _blob(&mut self, bounds: &Int) -> DecoderResult;

  fn _bool(&mut self) -> DecoderResult;

  fn _array(&mut self, bounds: &Int, typeid: &u8, event_allowed: bool) -> DecoderResult;

  fn _bitarray(&mut self, bounds: &Int) -> DecoderResult;

  fn _optional(&mut self, typeid: &u8, event_allowed: bool) -> DecoderResult;

  fn _fourcc(&mut self) -> DecoderResult;

  fn _choice(
    &mut self,
    bounds: &Int,
    fields: &Vec<(i64, (&str, u8))>,
    event_allowed: bool
  ) -> DecoderResult;

  fn _struct<'a>(&'a mut self, fields: &[Struct], event_allowed: bool) -> DecoderResult;
}

impl<'a> BitPackedDecoder<'a> {
  pub fn new(
    contents: Vec<u8>,
    typeinfos: &'a [ProtocolTypeInfo<'a>],
  ) -> BitPackedDecoder<'a> {
    let buffer = BitPackedBuffer::new(contents);

    BitPackedDecoder { buffer, typeinfos }
  }
}

impl Decoder for BitPackedDecoder<'_> {
  fn _int(&mut self, bounds: &Int) -> DecoderResult {
    let read = self.buffer.read_bits(bounds.1);
    DecoderResult::Value(bounds.0 + read as i64)
  }

  fn _blob(&mut self, bounds: &Int) -> DecoderResult {
    match self._int(bounds) {
      DecoderResult::Value(value) => DecoderResult::Blob(
        str::from_utf8(self.buffer.read_aligned_bytes(value as usize))
          .unwrap_or("")
          .to_string()
      ),
      _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
    }
  }

  fn _bool(&mut self) -> DecoderResult {
    match self._int(&Int(0, 1)) {
      DecoderResult::Value(value) => DecoderResult::Bool(value != 0),
      _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
    }
  }

  fn _array(&mut self, bounds: &Int, typeid: &u8, event_allowed: bool) -> DecoderResult {
    match self._int(bounds) {
      DecoderResult::Value(value) => {
        let mut array = Vec::with_capacity(value as usize);
        for _i in 0..value {
          let data = match self.instance(self.typeinfos, typeid, event_allowed) {
            DecoderResult::Value(value) => DecoderResult::DataFragment(value as u32),
            DecoderResult::Struct(values) => DecoderResult::Struct(values),
            _other => panic!("instance returned DecoderResult::{:?}", _other),
          };
          array.push(data);
        }

        DecoderResult::Array(array)
      }
      _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
    }
  }

  fn _bitarray(&mut self, bounds: &Int) -> DecoderResult {
    match self._int(bounds) {
      DecoderResult::Value(value) => {
        let bytes = self.buffer.read_bits(value as u8);
        // DecoderResult::Pair((value, bytes as i16))
        DecoderResult::Pair((0, 0))
      }
      _other => panic!("instance didn't return DecoderResult::Value {:?}", _other),
    }
  }

  fn _optional(&mut self, typeid: &u8, event_allowed: bool) -> DecoderResult {
    match self._bool() {
      DecoderResult::Bool(value) => {
        if value {
          self.instance(self.typeinfos, typeid, event_allowed)
        } else {
          DecoderResult::Null
        }
      }
      _other => panic!("_bool didn't return DecoderResult::Bool {:?}", _other),
    }
  }

  fn _fourcc(&mut self) -> DecoderResult {
    DecoderResult::Blob(self.buffer.read_unaligned_bytes(4))
  }

  fn _choice(
    &mut self,
    bounds: &Int,
    fields: &Vec<(i64, (&str, u8))>,
    event_allowed: bool,
  ) -> DecoderResult {
    let tag = match self._int(bounds) {
      DecoderResult::Value(value) => value,
      _other => panic!("_int didn't return DecoderResult::Value {:?}", _other),
    };

    match fields.iter().find(|(field_tag, _)| *field_tag == tag) {
      Some((_, field)) => {
        let choice_res = match self.instance(self.typeinfos, &field.1, event_allowed) {
          DecoderResult::Value(value) => value,
          _other => panic!("didn't find DecoderResult::Value"),
        };
        // println!("_choice instance returned {:?} {:?}", field.0, choice_res);
        match event_allowed {
          true => DecoderResult::Gameloop((field.0.to_owned(), choice_res)),
          false => DecoderResult::Empty,
        }
      },
      None => panic!("CorruptedError"),
    }
  }

  fn _struct<'a>(&mut self, fields: &[Struct], event_allowed: bool) -> DecoderResult {
    let mut result = Vec::with_capacity(fields.len());
    for field in fields {
      // appears that this isn't needed since field is never parent
      // match fields.into_iter().find(|f| f.2 as i64 == tag) {
      //   Some(field) => {
      //   if field.0 == "__parent" {
      //     let parent = self.instance(self.typeinfos, field.1);
      //   } else {
      // let field_value = match self.instance(self.typeinfos, field.1) {
      //   DecoderResult::Value(value) => value,
      //   _other => panic!("field.1 is not a value: {:?}", field),
      // };
      // result.insert(field.0.as_str(), field_value as u8);
      //   }
      //   },
      //   None => self._skip_instance(),
      // };

      // field always seems to exist?
      let field_value = self.instance(self.typeinfos, &field.1, event_allowed);
      match event_allowed {
        true => result.push((field.0.to_string(), field_value)),
        false => continue,
      }
    }

    DecoderResult::Struct(result)
  }
}

impl<'a> VersionedDecoder<'a> {
  pub fn new(
    contents: Vec<u8>,
    typeinfos: &'a [ProtocolTypeInfo<'a>],
  ) -> VersionedDecoder<'a> {
    let buffer = BitPackedBuffer::new(contents);

    VersionedDecoder { buffer, typeinfos }
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
    if skip == 0 {
      // array
      let length = self._vint();
      for _ in 0..length {
        self._skip_instance();
      }
    } else if skip == 1 {
      // bitblob
      let length = self._vint();
      self.buffer.read_aligned_bytes(((length + 7) / 8) as usize);
    } else if skip == 2 {
      // blob
      let length = self._vint();
      self.buffer.read_aligned_bytes(length as usize);
    } else if skip == 3 {
      // choice
      let tag = self._vint();
      self._skip_instance();
    } else if skip == 4 {
      // optional
      let exists = self.buffer.read_bits(8) != 0;
      if exists {
        self._skip_instance();
      }
    } else if skip == 5 {
      // struct
      let length = self._vint();
      for _ in 0..length {
        let tag = self._vint();
        self._skip_instance();
      }
    } else if skip == 6 {
      // u8
      self.buffer.read_aligned_bytes(1);
    } else if skip == 7 {
      // u32
      self.buffer.read_aligned_bytes(4);
    } else if skip == 8 {
      // u64
      self.buffer.read_aligned_bytes(8);
    } else if skip == 9 {
      // vint
      self._vint();
    }
  }
}

impl Decoder for VersionedDecoder<'_> {
  fn _int(&mut self, bounds: &Int) -> DecoderResult {
    self.expect_skip(9);
    DecoderResult::Value(self._vint())
  }

  fn _blob(&mut self, bounds: &Int) -> DecoderResult {
    self.expect_skip(2);
    let length = self._vint();
    DecoderResult::Blob(
      str::from_utf8(self.buffer.read_aligned_bytes(length as usize))
        .unwrap_or("")
        .to_string(),
    )
  }

  fn _bool(&mut self) -> DecoderResult {
    self.expect_skip(6);
    DecoderResult::Bool(self.buffer.read_bits(8) != 0)
  }

  fn _array(&mut self, bounds: &Int, typeid: &u8, event_allowed: bool) -> DecoderResult {
    self.expect_skip(0);
    let length = self._vint();

    let mut array = Vec::with_capacity(length as usize);
    for _ in 0..length {
      let data = match self.instance(self.typeinfos, typeid, event_allowed) {
        DecoderResult::Value(value) => DecoderResult::DataFragment(value as u32),
        DecoderResult::Struct(values) => DecoderResult::Struct(values),
        DecoderResult::Blob(value) => DecoderResult::Blob(value),
        _other => panic!("instance returned DecoderResult::{:?}", _other),
      };
      array.push(data);
    }

    DecoderResult::Array(array)
  }

  fn _bitarray(&mut self, bounds: &Int) -> DecoderResult {
    self.expect_skip(1);
    let length = self._vint();
    let bytes = self.buffer.read_aligned_bytes((length as usize + 7) / 8);
    let mut value: i16 = 0;
    for v in bytes {
      value += *v as i16;
    }
    // DecoderResult::Pair((length, value))
    DecoderResult::Pair((0, 0))
  }

  fn _optional(&mut self, typeid: &u8, event_allowed: bool) -> DecoderResult {
    self.expect_skip(4);
    if self.buffer.read_bits(8) != 0 {
      self.instance(self.typeinfos, typeid, event_allowed)
    } else {
      DecoderResult::Null
    }
  }

  fn _fourcc(&mut self) -> DecoderResult {
    self.expect_skip(7);
    DecoderResult::Blob(
      str::from_utf8(self.buffer.read_aligned_bytes(4))
        .unwrap_or("")
        .to_string()
    )
  }

  fn _choice(
    &mut self,
    bounds: &Int,
    fields: &Vec<(i64, (&str, u8))>,
    event_allowed: bool
  ) -> DecoderResult {
    self.expect_skip(3);
    let tag = self._vint();

    match fields.iter().find(|(field_tag, _)| *field_tag == tag) {
      Some((_, field)) => {
        let choice_res = match self.instance(self.typeinfos, &field.1, event_allowed) {
          DecoderResult::Value(value) => value,
          _other => panic!("didn't find DecoderResult::Value"),
        };
        // println!("_choice instance returned {:?} {:?}", field.0, choice_res);
        match event_allowed {
          true => DecoderResult::Gameloop((field.0.to_owned(), choice_res)),
          false => DecoderResult::Empty,
        }
      },
      None => {
        self._skip_instance();
        DecoderResult::Pair((0, 0))
      },
    }
  }

  fn _struct<'a>(&mut self, fields: &[Struct], event_allowed: bool) -> DecoderResult {
    self.expect_skip(5);
    let mut result = Vec::with_capacity(fields.len());
    let length = self._vint();
    for _ in 0..length {
      let tag = self._vint();

      // appears that this isn't needed since field is never parent
      // match fields.into_iter().find(|f| f.2 as i64 == tag) {
      //   Some(field) => {
      //   if field.0 == "__parent" {
      //     let parent = self.instance(self.typeinfos, field.1);
      //   } else {
      // let field_value = match self.instance(self.typeinfos, field.1) {
      //   DecoderResult::Value(value) => value,
      //   _other => panic!("field.1 is not a value: {:?}", field),
      // };
      // result.insert(field.0.as_str(), field_value as u8);
      //   }
      //   },
      //   None => self._skip_instance(),
      // };

      // field always seems to exist?
      let field = fields.iter().find(|f| f.2 as i64 == tag).unwrap();
      let field_value = self.instance(self.typeinfos, &field.1, event_allowed);
      match event_allowed {
        true => result.push((field.0.to_string(), field_value)),
        false => continue,
      }
    }

    DecoderResult::Struct(result)
  }
}
