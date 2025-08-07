use std::{io::Cursor, ops::Sub};

use anyhow::{Result, anyhow};
use bitstream_io::{BitRead, BitReader, LittleEndian};

#[derive(Debug)]
pub struct AIRSignature {
    pub magic: u32,
    pub version: u32,
    pub offset: u32,
    pub size: u32,
    pub cpu_type: u32,
    pub magic2: u32,
}

impl AIRSignature {
    pub fn new_from(
        magic: u32,
        version: u32,
        offset: u32,
        size: u32,
        cpu_type: u32,
        magic2: u32,
    ) -> Self {
        Self {
            magic,
            version,
            offset,
            size,
            cpu_type,
            magic2,
        }
    }
}

pub fn parse_apple_ir(content: &Vec<u8>) {
    let signature = AIRSignature::new_from(
        u32::from_le_bytes([content[0], content[1], content[2], content[3]]),
        u32::from_le_bytes([content[4], content[5], content[6], content[7]]),
        u32::from_le_bytes([content[8], content[9], content[10], content[11]]),
        u32::from_le_bytes([content[12], content[13], content[14], content[15]]),
        u32::from_le_bytes([content[16], content[17], content[18], content[19]]),
        u32::from_le_bytes([content[20], content[21], content[22], content[23]]),
    );

    let content_length = content.len();

    let cursor = Cursor::new(&content[24..]);
    let mut reader = BitReader::<_, LittleEndian>::new(cursor);

    let mut abbreviation_list: Vec<AIRAbbreviation> = vec![];
    let mut item_vec: Vec<AIRItem> = vec![];

    let mut parse = parse_abbreviation_id(&mut reader, 2, &mut abbreviation_list);

    println!("{:#?}", parse);

    parse = parse_abbreviation_id(&mut reader, 2, &mut abbreviation_list);

    println!("{:#?}", parse);

    parse = parse_abbreviation_id(&mut reader, 2, &mut abbreviation_list);

    println!("{:#?}", parse);
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    IDENTIFICATION = 13,
}

impl BlockType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            13 => Self::IDENTIFICATION,
            _ => todo!("{:?} not implemented.", v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AIRBlock {
    ty: BlockType,
    new_abbreviation_length: u32,
    block_length: u32,
    items: Vec<AIRItem>,
}

#[derive(Debug, Clone)]
pub enum AIRItem {
    Block(AIRBlock),
    Abbreviation(AIRAbbreviation),
    Record(AIRRecord),
    Uninitialized,
    EndBlock,
}

fn align_32(reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>) {
    let length = reader.position_in_bits().unwrap().clamp(0, 32);

    let missing = 32 - length;

    reader.skip(missing as u32).unwrap();
}

fn parse_enter_subblock(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    abbreviation_list: &mut Vec<AIRAbbreviation>,
) -> AIRBlock {
    let ty = BlockType::from_u8(read_vbr(reader, 8).unwrap() as u8);

    let new_abbreviation_length = read_vbr(reader, 4).unwrap() as u32;

    align_32(reader);

    let block_length = reader.read_var::<u32>(32).unwrap();

    let mut item = AIRItem::Uninitialized;
    let mut items: Vec<AIRItem> = vec![];

    while !matches!(item, AIRItem::EndBlock) {
        item = parse_abbreviation_id(reader, new_abbreviation_length, abbreviation_list);

        if !matches!(item, AIRItem::EndBlock) {
            items.push(item.clone());
        }
    }

    AIRBlock {
        ty,
        items,
        new_abbreviation_length,
        block_length,
    }
}

#[derive(Debug, Clone)]
pub struct AIRAbbreviation {
    operands: Vec<AIROperand>,
}

#[derive(Debug, Clone)]
pub enum AIROperand {
    Literal(u64),
    Fixed(u64),
    Variable(u64),
    Array(Box<AIROperand>),
    Char6,
    Blob,
}

fn parse_operand(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    operands_left: &mut u32,
) -> AIROperand {
    *operands_left -= 1;

    let is_literal = reader.read_var::<bool>(1).unwrap();

    match is_literal {
        true => AIROperand::Literal(reader.read_var::<u64>(8).unwrap()),
        false => {
            let operand_type = reader.read_var::<u8>(3).unwrap();

            let operand_type = match operand_type {
                1 => AIROperand::Fixed(reader.read_var::<u64>(5).unwrap()),
                2 => AIROperand::Variable(reader.read_var::<u64>(5).unwrap()),
                3 => AIROperand::Array(Box::new(parse_operand(reader, operands_left))),
                4 => AIROperand::Char6,
                5 => AIROperand::Blob,
                _ => todo!("\"{:?}\" not implemented.", operand_type),
            };

            operand_type
        }
    }
}

fn parse_define_abbreviation(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    abbreviation_list: &mut Vec<AIRAbbreviation>,
) -> AIRAbbreviation {
    let mut number_of_operands = reader.read_var::<u32>(5).unwrap();

    let mut operands: Vec<AIROperand> = vec![];

    println!("Length: {:?}", number_of_operands);

    while number_of_operands > 0 {
        operands.push(parse_operand(reader, &mut number_of_operands));
    }

    let result = AIRAbbreviation { operands };

    abbreviation_list.push(result.clone());

    result
}

#[derive(Debug, Clone)]
pub struct AIRRecord {
    id: u64,
    ops: AIROps,
}

fn read_vbr(reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>, width: u64) -> Result<u64> {
    if width < 1 || width > 32 {
        // This is `MaxChunkSize` in LLVM
        return Err(anyhow!("VBR Overflowed!"));
    }
    let test_bit = 1u64 << (width - 1);
    let mask = test_bit - 1;
    let mut res = 0;
    let mut offset = 0;
    loop {
        let next = reader.read_var::<u64>(width as u32)?;
        res |= (next & mask) << offset;
        offset += width - 1;
        // 64 may not be divisible by width
        if offset > 63 + width {
            return Err(anyhow!("VBR Overflowed!"));
        }
        if next & test_bit == 0 {
            break;
        }
    }
    Ok(res)
}

fn read_char6(reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>) -> Result<u64> {
    let value = reader.read_var::<u8>(6).unwrap();

    Ok(u64::from(match value {
        0..=25 => value + b'a',
        26..=51 => value + (b'a' - 26),
        52..=61 => value - (52 - b'0'),
        62 => b'.',
        63 => b'_',
        _ => return Err(anyhow!("Not a valid 6-bit character!")),
    }))
}

fn read_scalar_operand(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    op: &AIROperand,
) -> Result<u64> {
    Ok(match op {
        AIROperand::Literal(value) => *value,
        AIROperand::Fixed(width) => reader.read_var::<u64>(*width as u32)?,
        AIROperand::Variable(width) => read_vbr(reader, *width)?,
        AIROperand::Char6 => read_char6(reader)?,
        _ => {
            return Err(anyhow!(
                "`{:?}` is not a Scalar operand, use `read_payload_operand` instead.",
                op
            ));
        }
    })
}

#[derive(Debug, Clone)]
pub enum AIROps {
    Abbrev { state: usize, index: usize },
    Full(usize),
}

fn parse_external_record(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    abbreviation_list: &mut Vec<AIRAbbreviation>,
    index: usize,
) -> AIRRecord {
    let abbreviation = abbreviation_list[index].operands.first().unwrap();
    let id = read_scalar_operand(reader, abbreviation).unwrap();

    let ops = AIROps::Abbrev { state: 1, index };

    AIRRecord { id, ops }
}

fn parse_abbreviation_id(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    length: u32,
    abbreviation_list: &mut Vec<AIRAbbreviation>,
) -> AIRItem {
    let bit = reader.read_var::<u32>(length).unwrap();

    match bit {
        0 => {
            align_32(reader);
            AIRItem::EndBlock
        }
        1 => AIRItem::Block(parse_enter_subblock(reader, abbreviation_list)),
        2 => AIRItem::Abbreviation(parse_define_abbreviation(reader, abbreviation_list)),
        3 => todo!("UNABBREV_RECORD"),
        _ => AIRItem::Record(parse_external_record(
            reader,
            abbreviation_list,
            bit as usize - 4,
        )),
    }
}
