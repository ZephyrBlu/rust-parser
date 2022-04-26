mod protocol;
mod decoders;

use std::fs::File;
use std::io::SeekFrom;
use std::io::BufReader;
use std::io::prelude::*;
use std::str;
use std::collections::HashMap;
use std::io::copy;
// use bzip2::Decompress;
use bzip2_rs::DecoderReader;
// use bzip2_rs::ParallelDecoderReader;
// use bzip2_rs::RayonThreadPool;

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

#[derive(Debug, Copy, Clone)]
struct HashTableEntry {
    hash_a: u32,
    hash_b: u32,
    locale: u16,
    platform: u16,
    block_table_index: u32,
}

// MPQHashTableEntry.struct_format = '2I 2H I'

#[derive(Debug, Copy, Clone)]
struct BlockTableEntry {
    offset: u32,
    archived_size: usize,
    size: usize,
    flags: u32,
}

// MPQBlockTableEntry.struct_format = '4I'

#[derive(Debug)]
enum MPQTableEntry {
    Hash(HashTableEntry),
    Block(BlockTableEntry),
}

struct MPQArchive {
    file: BufReader<File>,
    header: MPQFileHeader,
    hash_table: Vec<MPQTableEntry>,
    block_table: Vec<MPQTableEntry>,
    encryption_table: HashMap<u64, u64>,
    compressed: Vec<u8>,
    decompressed_offsets: Vec<usize>,
    compression_type: u8,
}

impl MPQArchive {
    fn new(filename: &str) -> MPQArchive {
        let file = File::open(filename).expect("Failed to read replay file");
        let mut reader = BufReader::new(file);
        let header = MPQArchive::read_header(&mut reader);

        let encryption_table = MPQArchive::prepare_encryption_table();
        let hash_table = MPQArchive::read_table(&mut reader, &header, &encryption_table, "hash");
        let block_table = MPQArchive::read_table(&mut reader, &header, &encryption_table, "block");
        // let block_table_entry = MPQArchive::read_block_entry(
        //     "(listfile)",
        //     &encryption_table,
        //     &hash_table,
        //     &block_table,
        // ).expect("Couldn't find block table entry");
        // let contents = MPQArchive::_read_file(&mut reader, &header, &block_table_entry, false);
        let compressed = vec![];
        let decompressed_offsets = vec![];
        let compression_type = 0;

        MPQArchive {
            file: reader,
            header,
            hash_table,
            block_table,
            encryption_table,
            compressed,
            decompressed_offsets,
            compression_type,
        }
    }

    fn read_header(file: &mut BufReader<File>) -> MPQFileHeader {
        let mut magic = [0; 4];
        file.read_exact(&mut magic).unwrap();
        file.seek(SeekFrom::Start(0));

        match magic {
            MPQ_MAGIC_A => MPQArchive::read_mpq_header(magic, file, None),
            MPQ_MAGIC_B => {
                let user_data_header = MPQArchive::read_mpq_user_data_header(magic, file);
                MPQArchive::read_mpq_header(magic, file, Some(user_data_header))
            },
            _other => panic!("Invalid file header"),
        }
    }

    fn read_mpq_header(magic: [u8; 4], file: &mut BufReader<File>, user_data_header: Option<MPQUserDataHeader>) -> MPQFileHeader {
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

    fn read_mpq_user_data_header (magic: [u8; 4], file: &mut BufReader<File>) -> MPQUserDataHeader {
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

        MPQUserDataHeader {
            magic,
            user_data_size: u32::from_le_bytes(user_data_size),
            mpq_header_offset: u32::from_le_bytes(mpq_header_offset),
            user_data_header_size: user_data_header_size_value,
            content,
        }
    }

    fn read_table(file: &mut BufReader<File>, header: &MPQFileHeader, table: &HashMap<u64, u64>, table_entry_type: &str) -> Vec<MPQTableEntry> {
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
                    let archived_size = u32::from_le_bytes(table_entry[4..8].try_into().unwrap()) as usize;
                    let size = u32::from_le_bytes(table_entry[8..12].try_into().unwrap()) as usize;
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
            seed2 = (byte as u64 + seed1 + seed2 + (seed2 << 5) + 3) & 0xFFFFFFFF;
        }

        seed1
    }

    fn decrypt(table: &HashMap<u64, u64>, data: &[u8], key: u64) -> Vec<u8> {
        let mut seed1: u64 = key;
        let mut seed2: u64 = 0xEEEEEEEE;
        let mut result = vec![];

        for i in 0..(data.len() / 4) {
            seed2 += table[&(0x400 + (seed1 & 0xFF))] as u64;
            seed2 &= 0xFFFFFFFF;

            let position = i * 4;
            let value_bytes: [u8; 4] = (&data[position..position + 4]).try_into().unwrap();
            let mut value = u32::from_le_bytes(value_bytes) as u64;
            value = (value ^ (seed1 + seed2)) & 0xFFFFFFFF;

            seed1 = ((!seed1 << 0x15) + 0x11111111) | (seed1 >> 0x0B);
            seed1 &= 0xFFFFFFFF;
            seed2 = (value + seed2 + (seed2 << 5) + 3) & 0xFFFFFFFF;

            let packed_value: u32 = value.try_into().unwrap();
            result.extend(packed_value.to_le_bytes());
        }

        result
    }

    fn _read_file(
        file: &mut BufReader<File>,
        header: &MPQFileHeader,
        block_entry: &BlockTableEntry,
        force_decompress: bool
    ) -> Option<Vec<u8>> {
        if block_entry.flags & MPQ_FILE_EXISTS != 0 {
            if block_entry.archived_size == 0 {
                return None;
            }

            let offset = block_entry.offset + header.offset;
            file.seek(SeekFrom::Start(offset as u64));

            let mut file_data = vec![0; block_entry.archived_size as usize];
            file.read_exact(&mut file_data).unwrap();

            if block_entry.flags & MPQ_FILE_ENCRYPTED != 0 {
                panic!("Encrpytion not supported");
            }

            // file has many sectors that need to be separately decompressed
            if block_entry.flags & MPQ_FILE_SINGLE_UNIT == 0 {
                panic!("Not implemented yet");
                // let sector_size = 512 << header.sector_size_shift;
                // let mut sectors = block_entry.size / sector_size + 1;

                // let mut crc = false;
                // if block_entry.flags & MPQ_FILE_SECTOR_CRC != 0 {
                //     crc = true;
                //     sectors += 1;
                // }

                // let positions = file_data[..4 * (sectors + 1)];
                // let mut result = vec![];
                // let mut sector_bytes_left = block_entry.size;

                // for i in 0..(positions.len() - (crc ? 2 : 1)) {
                //     let sector = file_data[positions[i]..positions[i + 1]]
                // }
            } else if (
                block_entry.flags & MPQ_FILE_COMPRESS != 0 &&
                (force_decompress || block_entry.size > block_entry.archived_size)
            ) {
                file_data = MPQArchive::decompress(file_data);
            }

            return Some(file_data);
        }
        None
    }

    fn read_block_entry(
        archive_filename: &str,
        encryption_table: &HashMap<u64, u64>,
        hash_table: &[MPQTableEntry],
        block_table: &[MPQTableEntry],
    ) -> Option<BlockTableEntry> {
        let hash_entry_wrapper = MPQArchive::get_hash_table_entry(encryption_table, hash_table, archive_filename);
        let hash_entry = match hash_entry_wrapper {
            Some(entry) => entry,
            None => return None,
        };

        match &block_table[hash_entry.block_table_index as usize] {
            MPQTableEntry::Block(entry) => Some(*entry),
            _other => panic!("Not block entry"),
        }
    }

    fn read_file(&mut self, archive_filename: &str) -> Option<Vec<u8>> {
        // let file = File::open(self.filename).expect("Failed to read replay file");
        // let mut reader = BufReader::new(file);
        let block_table_entry = MPQArchive::read_block_entry(
            archive_filename,
            &self.encryption_table,
            &self.hash_table,
            &self.block_table,
        ).expect("Couldn't find block table entry");
        let force_decompress = false;

        MPQArchive::_read_file(
            &mut self.file,
            &self.header,
            &block_table_entry,
            force_decompress,
        )
    }

    fn get_hash_table_entry(encryption_table: &HashMap<u64, u64>, hash_table: &[MPQTableEntry], filename: &str) -> Option<HashTableEntry> {
        let hash_a = MPQArchive::hash(encryption_table, filename, MPQHash::HashA);
        let hash_b = MPQArchive::hash(encryption_table, filename, MPQHash::HashB);

        for entry in hash_table {
            if let MPQTableEntry::Hash(table_entry) = entry {
                if (table_entry.hash_a as u64) == hash_a && (table_entry.hash_b as u64) == hash_b {
                    return Some(*table_entry);
                }
            };
        }

        None
    }

    fn decompress(data: Vec<u8>) -> Vec<u8> {
        let compression_type = data[0];

        if compression_type == 0 {
            data
        } else if compression_type == 2 {
            panic!("zlib compression not implemented yet");
        } else if  compression_type == 16 {
            // let mut decompressor = Decompress::new(false);
            // decompressor.decompress_vec(&mut &data[1..], &mut decompressed_data).unwrap();

            // let mut reader = ParallelDecoderReader::new(Cursor::new(data), RayonThreadPool, usize::max_value());
            // copy(&mut reader, output);

            let mut decompressed_data = vec![];
            let mut reader = DecoderReader::new(&data[1..]);
            copy(&mut reader, &mut decompressed_data);

            decompressed_data
        } else {
            panic!("Unsupported compression type")
        }
    }
}

// mpyq archive decoder
// sc2 protocol
// bit decoder

use std::time::Instant;

// #[global_allocator]
// static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let now = Instant::now();
    let mut archive = MPQArchive::new("neural parasite upgrade.SC2Replay");
    // let mut archive = MPQArchive::new("big replay.SC2Replay");
    // println!("read MPQ archive {:.2?}", now.elapsed());

    let header_content = &archive.header.user_data_header.as_ref().expect("No user data header").content;
    // println!("read header {:.2?}", now.elapsed());

    let contents = archive.read_file("replay.tracker.events").unwrap();
    println!("read tracker events {:.2?}", now.elapsed());

    let game_info = archive.read_file("replay.game.events").unwrap();
    println!("read game events {:.2?}", now.elapsed());

    let init_data = archive.read_file("replay.initData").unwrap();
    println!("read details {:.2?}", now.elapsed());

    let metadata = archive.read_file("replay.gamemetadata.json").unwrap();
    let string = str::from_utf8(&metadata).unwrap();
    println!("read metadata {:.2?}", now.elapsed());

    let details = archive.read_file("replay.details").unwrap();

    println!("files parsed {:.2?}", now.elapsed());
    let protocol = protocol::Protocol::new();
    println!("protocol instantiated {:.2?}", now.elapsed());
    
    let tracker_events = protocol.decode_replay_tracker_events(contents);
    println!("decoded replay tracker events {:.2?}", now.elapsed());

    let game_events = protocol.decode_replay_game_events(game_info);
    println!("decoding replay game events {:.2?}", now.elapsed());

    println!("protocol parsed {:.2?}", now.elapsed());
}
