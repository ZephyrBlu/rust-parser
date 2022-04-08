use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use std::str;
use std::collections::HashMap;

const MPQ_FILE_IMPLODE: u32        = 0x00000100;
const MPQ_FILE_COMPRESS: u32       = 0x00000200;
const MPQ_FILE_ENCRYPTED: u32      = 0x00010000;
const MPQ_FILE_FIX_KEY: u32        = 0x00020000;
const MPQ_FILE_SINGLE_UNIT: u32    = 0x01000000;
const MPQ_FILE_DELETE_MARKER: u32  = 0x02000000;
const MPQ_FILE_SECTOR_CRC: u32     = 0x04000000;
const MPQ_FILE_EXISTS: u32         = 0x80000000;

const MPQ_MAGIC_A: [u8; 4] = [77, 80, 81, 26];
const MPQ_MAGIC_B: [u8; 4] = [77, 80, 81, 27];

#[derive(Copy, Clone)]
enum MPQHash {
    TableOffset = 0,
    HashA = 1,
    HashB = 2,
    Table = 3,
}

struct MPQFileHeader {
    magic: [u8; 4],
    offset: u32,
    header_size: u32,
    archive_size: u32,
    format_version: u16,
    sector_size_shift: u16,
    hash_table_offset: u32,
    block_table_offset: u32,
    hash_table_entries: u32,
    block_table_entries: u32,
    user_data_header: Option<MPQUserDataHeader>,
    extended: Option<MPQFileHeaderExt>,
}


// MPQFileHeader.struct_format = '< 4s 2I 2H 4I' = 4 + 8 + 4 + 16 = 32 bytes

struct MPQFileHeaderExt {
    extended_block_table_offset: i64,
    hash_table_offset_high: i16,
    block_table_offset_high: i16,
}


// MPQFileHeaderExt.struct_format = 'q 2h'

struct MPQUserDataHeader {
    magic: [u8; 4],
    user_data_size: u32,
    mpq_header_offset: u32,
    user_data_header_size: u32,
    content: Vec<u8>,
}

// MPQUserDataHeader.struct_format = '< 4s 3I'

struct HashTableEntry {
    hash_a: u32,
    hash_b: u32,
    locale: u16,
    platform: u16,
    block_table_index: u32,
}

// MPQHashTableEntry.struct_format = '2I 2H I'

struct BlockTableEntry {
    offset: u32,
    archived_size: u32,
    size: u32,
    flags: u32,
}

// MPQBlockTableEntry.struct_format = '4I'

enum MPQTableEntry {
    Hash(HashTableEntry),
    Block(BlockTableEntry),
}

struct MPQArchive {
    file: File,
    header: MPQFileHeader,
    hash_table: Vec<MPQTableEntry>,
    block_table: Vec<MPQTableEntry>,
    files: Vec<u8>,
}

impl MPQArchive {
    fn new(filename: &str) -> MPQArchive {
        let mut file = File::open(filename).expect("Failed to read replay file");
        let header = MPQArchive::read_header(&mut file);

        let encryption_table = MPQArchive::prepare_encryption_table();
        let hash_table = MPQArchive::read_table(&mut file, &header, &encryption_table, "hash");
        let block_table = MPQArchive::read_table(&mut file, &header, &encryption_table, "block");

        let files = MPQArchive::read_file("(listfile)", false);

        MPQArchive {
            file,
            header,
            hash_table: vec![],
            block_table: vec![],
            files: vec![],
        }
    }

    fn read_header(file: &mut File) -> MPQFileHeader {
        let mut magic = [0; 4];
        file.read_exact(&mut magic).unwrap();

        match magic {
            MPQ_MAGIC_A => MPQArchive::read_mpq_header(magic, file, None),
            MPQ_MAGIC_B => {
                println!("");
                let user_data_header = MPQArchive::read_mpq_user_data_header(magic, file);
                MPQArchive::read_mpq_header(magic, file, Some(user_data_header))
            },
            _other => panic!("Invalid file header"),
        }
    }

    fn read_mpq_header(magic: [u8; 4], file: &mut File, user_data_header: Option<MPQUserDataHeader>) -> MPQFileHeader {
        let mut header_size = [0; 4];
        let mut archive_size = [0; 4];
        let mut format_version = [0; 2];
        let mut sector_size_shift = [0; 2];
        let mut hash_table_offset = [0; 4];
        let mut block_table_offset = [0; 4];
        let mut hash_table_entries = [0; 4];
        let mut block_table_entries = [0; 4];

        let offset = match user_data_header.as_ref() {
            Some(header) => header.mpq_header_offset,
            None => 0,
        };
        file.seek(SeekFrom::Start(offset as u64 + 4)).expect("Failed to seek");

        file.read_exact(&mut header_size).unwrap();
        file.read_exact(&mut archive_size).unwrap();
        file.read_exact(&mut format_version).unwrap();
        file.read_exact(&mut sector_size_shift).unwrap();
        file.read_exact(&mut hash_table_offset).unwrap();
        file.read_exact(&mut block_table_offset).unwrap();
        file.read_exact(&mut hash_table_entries).unwrap();
        file.read_exact(&mut block_table_entries).unwrap();

        let mut header_extension = None;
        let format_version_value = u16::from_le_bytes(format_version);

        if format_version_value == 1 {
            let mut extended_block_table_offset = [0; 8];
            let mut hash_table_offset_high = [0; 2];
            let mut block_table_offset_high = [0; 2];

            file.read_exact(&mut extended_block_table_offset).unwrap();
            file.read_exact(&mut hash_table_offset_high).unwrap();
            file.read_exact(&mut block_table_offset_high).unwrap();

            header_extension = Some(MPQFileHeaderExt {
                extended_block_table_offset: i64::from_ne_bytes(extended_block_table_offset),
                hash_table_offset_high: i16::from_ne_bytes(hash_table_offset_high),
                block_table_offset_high: i16::from_ne_bytes(block_table_offset_high),
            });
        }

        MPQFileHeader {
            magic,
            offset,
            header_size: u32::from_le_bytes(header_size),
            archive_size: u32::from_le_bytes(archive_size),
            format_version: format_version_value,
            sector_size_shift: u16::from_le_bytes(sector_size_shift),
            hash_table_offset: u32::from_le_bytes(hash_table_offset),
            block_table_offset: u32::from_le_bytes(block_table_offset),
            hash_table_entries: u32::from_le_bytes(hash_table_entries),
            block_table_entries: u32::from_le_bytes(block_table_entries),
            user_data_header,
            extended: header_extension,
        }
    }

    fn read_mpq_user_data_header (magic: [u8; 4], file: &mut File) -> MPQUserDataHeader {
        let mut user_data_size = [0; 4];
        let mut mpq_header_offset = [0; 4];
        let mut user_data_header_size = [0; 4];

        file.seek(SeekFrom::Start(4));

        file.read_exact(&mut user_data_size).unwrap();
        file.read_exact(&mut mpq_header_offset).unwrap();
        file.read_exact(&mut user_data_header_size).unwrap();

        let user_data_header_size_value = u32::from_le_bytes(user_data_header_size);
        let mut content = vec![0; user_data_header_size_value as usize];
        file.read_exact(&mut content).unwrap();
        println!("file content{:?}", content);

        MPQUserDataHeader {
            magic,
            user_data_size: u32::from_le_bytes(user_data_size),
            mpq_header_offset: u32::from_le_bytes(mpq_header_offset),
            user_data_header_size: user_data_header_size_value,
            content,
        }
    }

    fn read_table(file: &mut File, header: &MPQFileHeader, table: &HashMap<u64, u64>, table_entry_type: &str) -> Vec<MPQTableEntry> {
        let (table_offset, table_entries, key) = match table_entry_type {
            "hash" => (
                header.hash_table_offset,
                header.hash_table_entries,
                MPQArchive::hash(table, "(hash table)", MPQHash::Table),
            ),
            "block" => (
                header.block_table_offset,
                header.block_table_entries,
                MPQArchive::hash(table, "(block table)", MPQHash::Table),
            ),
            _other => panic!("Neither block or header"),
        };

        let file_offset: u32 = table_offset + header.offset;
        file.seek(SeekFrom::Start(file_offset as u64));

        let mut data = vec![0; (table_entries * 16) as usize];
        file.read_exact(&mut data).unwrap();
        let decrypted_data = MPQArchive::decrypt(table, &data, key);

        let mut table_values = Vec::with_capacity(table_entries as usize);
        for i in 0..table_entries {
            let position = (i * 16) as usize;
            let table_entry: [u8; 16] = (&decrypted_data[position..position + 16]).try_into().unwrap();
            let entry_value = match table_entry_type {
                "hash" => {
                    let hash_a = u32::from_le_bytes(table_entry[0..4].try_into().unwrap());
                    let hash_b = u32::from_le_bytes(table_entry[4..8].try_into().unwrap());
                    let locale = u16::from_le_bytes(table_entry[8..10].try_into().unwrap());
                    let platform = u16::from_le_bytes(table_entry[10..12].try_into().unwrap());
                    let block_table_index = u32::from_le_bytes(table_entry[12..16].try_into().unwrap());

                    MPQTableEntry::Hash(HashTableEntry {
                        hash_a,
                        hash_b,
                        locale,
                        platform,
                        block_table_index,
                    })
                },
                "block" => {
                    let offset = u32::from_le_bytes(table_entry[0..4].try_into().unwrap());
                    let archived_size = u32::from_le_bytes(table_entry[4..8].try_into().unwrap());
                    let size = u32::from_le_bytes(table_entry[8..12].try_into().unwrap());
                    let flags = u32::from_le_bytes(table_entry[12..16].try_into().unwrap());

                    MPQTableEntry::Block(BlockTableEntry {
                        offset,
                        archived_size,
                        size,
                        flags,
                    })
                },
                _other => panic!("Neither block or header"),
            };
            table_values.push(entry_value);
        }

        table_values
    }

    fn prepare_encryption_table() -> HashMap<u64, u64> {
        let mut seed: u64 = 0x00100001;
        let mut encryption_table = HashMap::new();

        for i in 0..256 {
            let mut index = i;
            for _j in 0..5 {
                seed = (seed * 125 + 3) % 0x2AAAAB;
                let temp1 = (seed & 0xFFFF) << 0x10;

                seed = (seed * 125 + 3) % 0x2AAAAB;
                let temp2 = seed & 0xFFFF;

                encryption_table.insert(index, temp1 | temp2);

                index += 0x100;
            }
        }

        encryption_table
    }

    fn hash(table: &HashMap<u64, u64>, string: &str, hash_type: MPQHash) -> u64 {
        let mut seed1: u64 = 0x7FED7FED;
        let mut seed2: u64 = 0xEEEEEEEE;

        for byte in string.to_uppercase().bytes() {
            let value: u64 = table[&(((hash_type as u64) << 8) + byte as u64)];
            seed1 = (value ^ (seed1 + seed2)) & 0xFFFFFFFF;
            seed2 = byte as u64 + seed1 + seed2 + (seed2 << 5) + 3 & 0xFFFFFFFF;
        }

        seed1
    }

    fn decrypt(table: &HashMap<u64, u64>, data: &Vec<u8>, key: u64) -> Vec<u8> {
        let mut seed1: u64 = key;
        let mut seed2: u64 = 0x00100001;
        let mut result = vec![];

        println!("data length {:?} {:?}", data.len(), data.len() / 4);

        for i in 0..(data.len() / 4) {
            seed2 += table[&(0x400 + (seed1 & 0xFF))] as u64;
            seed2 &= 0xFFFFFFFF;

            let position = i * 4;
            let value_bytes: [u8; 4] = (&data[position..position + 4]).try_into().unwrap();
            let mut value = u32::from_le_bytes(value_bytes) as u64;
            println!("value before {:?}", value);
            value = (value ^ (seed1 + seed2)) & 0xFFFFFFFF;

            seed1 = ((!seed1 << 0x15) + 0x11111111) | (seed1 >> 0x0B);
            seed1 &= 0xFFFFFFFF;
            seed2 = value + seed2 + (seed2 << 5) + 3 & 0xFFFFFFFF;

            result.extend(value.to_le_bytes());
        }

        result
    }

    fn read_file(filename: &str, force_decompress: bool) -> Vec<u8> {
        vec![]
    }

    fn decompress(data: Vec<u8>) -> Vec<u8> {
        data
    }
}

// mpyq archive decoder
// sc2 protocol
// bit decoder

use std::time::Instant;

fn main() {
    let now = Instant::now();
    let replay = MPQArchive::new("neural parasite upgrade.SC2Replay");
    println!("{:.2?}", now.elapsed());
    println!("file header {:?}", replay.header.header_size);
}

