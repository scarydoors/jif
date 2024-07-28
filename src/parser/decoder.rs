#![allow(dead_code)]

use super::lzw;

use thiserror::Error;
use anyhow::{anyhow, Result};
use log::debug;

use std::io::prelude::*;
use std::str;
use std::fmt::Debug;

const EXTENSION_INTRODUCER: u8 = 0x21;
const IMAGE_DESCRIPTOR_LABEL: u8 = 0x2c;
const TRAILER_LABEL: u8 = 0x3b;

// Extension labels
const APPLICATION_EXTENSION: u8 = 0xff;
const COMMENT_EXTENSION: u8 = 0xfe;
const GRAPHIC_CONTROL_EXTENSION: u8 = 0xf9;
const PLAIN_TEXT_EXTENSION: u8 = 0x01;

#[derive(Debug)]
enum ExtensionType {
    Application,
    Comment,
    GraphicControl,
    PlainText,
}

impl TryFrom<u8> for ExtensionType {
    type Error = ParserError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        use ExtensionType::*;

        match value {
            APPLICATION_EXTENSION => Ok(Application),
            COMMENT_EXTENSION => Ok(Comment),
            GRAPHIC_CONTROL_EXTENSION => Ok(GraphicControl),
            PLAIN_TEXT_EXTENSION => Ok(PlainText),

            _ => Err(ParserError::InvalidExtensionLabel(value))
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub(crate) struct GraphicControlExtension {
    disposal_method: u8,
    user_input_flag: bool,
    transparent_color_flag: bool,

    delay_time: u16,
    transparent_color_index: u8,
}

#[derive(Debug)]
pub(crate) struct TableBasedImage {
    // includes image descriptor inline
    pub(crate) left_position: u16,
    pub(crate) top_position: u16,

    pub(crate) width: u16,
    pub(crate) height: u16,

    pub(crate) local_color_table_flag: bool,
    pub(crate) interlace_flag: bool,
    pub(crate) sort_flag: bool,
    pub(crate) local_color_table_size: Option<u32>,

    pub(crate) local_color_table: Option<Box<[u8]>>,

    pub(crate) image_indexes: Option<Box<[u8]>>,
}

#[derive(Debug)]
pub(crate) struct GraphicBlock {
    pub(crate) extension: Option<GraphicControlExtension>,
    pub(crate) render_block: TableBasedImage,
}

#[derive(Debug)]
pub(crate) enum SpecialPurposeExtension {
    ApplicationBlock {
        application_identifier: Box<str>,
        application_authentication_code: Box<[u8]>,
        application_data: Box<[u8]>,
    },
    CommentBlock(Box<[u8]>)
}

#[derive(Debug)]
pub(crate) enum Version {
    V87a,
    V89a
}

impl TryFrom<&str> for Version {
    type Error = ParserError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "87a" => Ok(Version::V87a),
            "89a" => Ok(Version::V89a),
            version => Err(ParserError::UnsupportedVersion(version.into()))
        }
    }
}

#[derive(Debug)]
enum LoopCount {
    Infinite,
    Number(u16),
}

#[derive(Debug)]
pub(crate) struct LogicalScreenDescriptor {
    pub(crate) screen_width: u16,
    pub(crate) screen_height: u16,
    pub(crate) global_color_table_flag: bool,
    pub(crate) color_resolution: u8,
    pub(crate) sort_flag: bool,
    pub(crate) global_color_table_size: Option<u32>,
    pub(crate) background_color_index: u8,
    pub(crate) pixel_aspect_ratio: u8,
}

#[derive(Debug)]
pub(crate) enum ParserState {
    ProcessMagic,
    ProcessLogicalScreenDescriptor,
    ProcessGlobalColorTable,
    ProcessTrailer,

    DetermineNextBlock(Option<GraphicControlExtension>),
    ProcessExtension(u8),
    ProcessImageDescriptor(Option<GraphicControlExtension>),
    ProcessLocalColorTable(GraphicBlock),
    ProcessImageData(GraphicBlock),

    Done,
}

#[derive(Error, Debug)]
pub(crate) enum ParserError {
    #[error("signature is invalid")]
    InvalidSignature,

    #[error("version {0} in the header is unsupported")]
    UnsupportedVersion(String),

    #[error("encountered extension with label 0x{0:02x}, this label is not supported")]
    InvalidExtensionLabel(u8),

    #[error("expected image descriptor after graphic control extension")]
    ExpectedImageDescriptor,

    #[error("encountered unexpected label, this label is not supported: {0}")]
    UnexpectedLabel(u8),

    #[error("encountered application descriptor with name {name}, expected descriptor data length to be {expected}, actual length is {actual}")]
    UnexpectedApplicationDescriptorDataLength {
        name: String,
        expected: usize,
        actual: usize,
    }
}

#[derive(Debug)]
pub struct Decoder<'a, T: Read> {
    inner: &'a mut T,
    pub(crate) version: Option<Version>,
    pub(crate) logical_screen_descriptor: Option<LogicalScreenDescriptor>,
    pub(crate) global_color_table: Option<Box<[u8]>>,
    pub(crate) special_purpose_extensions: Vec<SpecialPurposeExtension>,
    pub(crate) graphic_blocks: Vec<GraphicBlock>,
    loop_count: Option<LoopCount>,
}

impl<'a, T: Read + Debug> Decoder<'a, T> {
    pub fn new(inner: &'a mut T) -> Self {
        Self {
            inner,
            version: None,
            logical_screen_descriptor: None,
            global_color_table: None,
            special_purpose_extensions: Vec::new(),
            graphic_blocks: Vec::new(),
            loop_count: None,
        }
    }

    pub fn parse(&mut self) -> Result<()> {
        let mut state = ParserState::ProcessMagic;

        loop {
            debug!("begin parsing state {:?}", state);

            state = self.process_next_state(state)?;
            if let ParserState::Done = state {
                break Ok(());
            }
        }
    }

    fn process_next_state(&mut self, next_state: ParserState) -> Result<ParserState>  {
        use ParserState::*;

        match next_state {
            ProcessMagic => {
                let signature = self.read_str(3)?;
                if signature.as_ref() != "GIF" {
                    return Err(ParserError::InvalidSignature.into())
                }
                debug!("processed signature, got GIF");

                self.version = Some(Version::try_from(self.read_str(3)?.as_ref())?);
                debug!("processed version, got {:?}", self.version);

                Ok(ProcessLogicalScreenDescriptor)
            },
            ProcessLogicalScreenDescriptor => {
                let screen_width = self.read_u16()?;
                let screen_height = self.read_u16()?;

                let packed_fields = self.read_byte()?;

                // packed field start
                let global_color_table_flag = packed_fields & 0b10000000 != 0;
                let color_resolution = (packed_fields >> 4) & 0b00000111;
                let sort_flag = packed_fields & 0b00001000 != 0;
                let global_color_table_size = if global_color_table_flag {
                    Some(3 * 2_u32.pow(((packed_fields & 0b00000111) + 1).into()))
                } else {
                    None
                };
                // packed field end

                let background_color_index = self.read_byte()?;
                let pixel_aspect_ratio = self.read_byte()?;

                self.logical_screen_descriptor = Some(LogicalScreenDescriptor {
                    screen_height,
                    screen_width,
                    global_color_table_flag,
                    color_resolution,
                    sort_flag,
                    global_color_table_size,
                    background_color_index,
                    pixel_aspect_ratio,
                });

                debug!("processed logical screen descriptor, got: {:#?}", self.logical_screen_descriptor);

                let next_state = if global_color_table_flag {
                    ProcessGlobalColorTable
                } else {
                    DetermineNextBlock(None)
                };

                Ok(next_state)
            },
            ProcessGlobalColorTable => {
                let screen_desc = self.logical_screen_descriptor.as_ref().expect("logical screen descriptor should not be none");
                let size = screen_desc.global_color_table_size.expect("global color table size should not be none");

                self.global_color_table = Some(self.read_bytes(size as usize)?);
                debug!("processed global color table, got: {:#?}", self.global_color_table);

                Ok(DetermineNextBlock(None))
            },
            ProcessTrailer => {
                Ok(Done)
            }
            DetermineNextBlock(graphic_control_extension) => {
                let introducer_or_label = self.read_byte()?;

                match introducer_or_label {
                    // extension introducer means that a label follows determining what exact type
                    // of extension it is.
                    EXTENSION_INTRODUCER => Ok(ProcessExtension(self.read_byte()?)),
                    IMAGE_DESCRIPTOR_LABEL => Ok(ProcessImageDescriptor(graphic_control_extension)),
                    TRAILER_LABEL => Ok(ProcessTrailer),
                    label => Err(ParserError::UnexpectedLabel(label).into())
                }
            },
            ProcessExtension(label) => self.process_extension(ExtensionType::try_from(label)?),
            ProcessImageDescriptor(graphic_control_extension) => {
                let left_position = self.read_u16()?;
                let top_position = self.read_u16()?;

                let width = self.read_u16()?;
                let height = self.read_u16()?;

                let packed_fields = self.read_byte()?;

                let local_color_table_flag = packed_fields & 0b10000000 != 0;
                let interlace_flag = packed_fields & 0b01000000 != 0;
                let sort_flag = packed_fields & 0b00100000 != 0;
                // TODO: this should be optional!!
                let local_color_table_size = if local_color_table_flag {
                    Some(3 * 2_u32.pow(((packed_fields & 0b00000111) + 1).into()))
                } else {
                    None
                };

                let graphic_block = GraphicBlock {
                    extension: graphic_control_extension,
                    render_block: TableBasedImage {
                        left_position,
                        top_position,
                        width,
                        height,
                        local_color_table_flag,
                        interlace_flag,
                        sort_flag,
                        local_color_table_size,

                        local_color_table: None,
                        image_indexes: None,
                    }
                };

                let next_state = if local_color_table_flag {
                    ProcessLocalColorTable(graphic_block)
                } else {
                    ProcessImageData(graphic_block)
                };

                Ok(next_state)
            },
            ProcessLocalColorTable(mut graphic_block) => {
                let size = graphic_block.render_block.local_color_table_size.expect("global color table size should not be none");

                graphic_block.render_block.local_color_table = Some(self.read_bytes(size as usize)?);

                Ok(ProcessImageData(graphic_block))
            },
            ProcessImageData(mut graphic_block) => {
                let lzw_code_size = self.read_byte()?;
                let data_stream = self.read_data_sub_blocks()?;

                let indicies = lzw::lzw_decode(&data_stream, lzw_code_size.into());
                graphic_block.render_block.image_indexes = Some(indicies.into_boxed_slice());

                self.graphic_blocks.push(graphic_block);

                Ok(DetermineNextBlock(None))
            },
            _ => {
                unimplemented!();
            }
        }
    }

    fn process_extension(&mut self, label: ExtensionType) -> Result<ParserState> {
        use ExtensionType::*;

        debug!("processing extension type: {:?}", label);
        match label {
            Application => {
                let block_size = self.read_byte()?;
                debug_assert_eq!(block_size, 11);
                let application_identifier = self.read_str(8)?;

                let application_authentication_code = self.read_bytes(3)?;
                let application_data = self.read_data_sub_blocks()?;

                if application_identifier.as_ref() == "NETSCAPE" && application_authentication_code.as_ref() == "2.0".as_bytes() {
                    // PERF: we check the length twice essentially with this and the try_into below, make it so it only happens once.
                    if application_data.len() != 3 {
                        return Err(
                            ParserError::UnexpectedApplicationDescriptorDataLength {
                                name: application_identifier.into(),
                                expected: 3,
                                actual: application_data.len()
                            }.into()
                        );
                    }

                    debug_assert_eq!(1, application_data[0]);

                    let loop_number = u16::from_le_bytes(application_data[1..3].try_into()?);
                    self.loop_count = Some(
                        match loop_number {
                            0 => LoopCount::Infinite,
                            number => LoopCount::Number(number)
                        }
                    );
                };

                self.special_purpose_extensions.push(
                    SpecialPurposeExtension::ApplicationBlock {
                        application_identifier,
                        application_authentication_code,
                        application_data
                    }
                );
                debug!("processed application block, got: {:#?}", self.special_purpose_extensions.last());
                Ok(ParserState::DetermineNextBlock(None))
            },
            Comment => {
                // sequence of data sub-blocks
                let data = self.read_data_sub_blocks()?;
                debug!("processed comment block, got: {}", String::from_utf8_lossy(&data));
                self.special_purpose_extensions.push(
                    SpecialPurposeExtension::CommentBlock(data)
                );
                Ok(ParserState::DetermineNextBlock(None))
            },
            GraphicControl => {
                let block_size = self.read_byte()?;
                debug_assert_eq!(block_size, 4);

                let packed_fields = self.read_byte()?;
                // packed fields definition
                // XXXYYYZW
                // XXX = reserved, not needed
                // YYY = disposal method, indicates what to do with graphic after displaying
                // Z = user input flag
                // W = transparent color flag

                let disposal_method = (packed_fields >> 2) & 0b00000111;
                let user_input_flag = packed_fields & 0b00000010 != 0;
                let transparent_color_flag = packed_fields & 0b00000001 != 0;

                let delay_time = self.read_u16()?;
                let transparent_color_index = self.read_byte()?;

                let block_terminator = self.read_byte()?;
                assert_eq!(block_terminator, 0);

                let graphic_control_extension = GraphicControlExtension {
                    disposal_method,
                    user_input_flag,
                    transparent_color_flag,

                    delay_time,
                    transparent_color_index
                };

                debug!("processed GraphicControlExtension: {:#?}", graphic_control_extension);

                Ok(ParserState::DetermineNextBlock(Some(graphic_control_extension)))
            },
            PlainText => {
                // i do not want to support this right now...
                let block_size = self.read_byte()?;
                debug_assert_eq!(block_size, 12);

                // skip data portion
                self.read_bytes(12)?;

                self.read_data_sub_blocks()?;

                Ok(ParserState::DetermineNextBlock(None))
            },
        }
    }

    fn read_bytes(&mut self, count: usize) -> Result<Box<[u8]>> {
        let mut buffer = vec![0; count];
        self.inner.read_exact(&mut buffer)?;
        Ok(buffer.into_boxed_slice())
    }

    fn read_byte(&mut self) -> Result<u8> {
        let mut buffer: [u8; 1] = [0; 1];
        self.inner.read_exact(&mut buffer)?;
        Ok(u8::from_le_bytes(buffer))
    }

    fn read_u16(&mut self) -> Result<u16> {
        // spec: Unless otherwise stated, multi-byte numeric fields are ordered with the Least
        // Significant Byte first.

        let mut buffer: [u8; 2] = [0; 2];
        self.inner.read_exact(&mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    fn read_str(&mut self, count: usize) -> Result<Box<str>> {
        let mut buffer = vec![0; count];
        self.inner.read(&mut buffer)?;
        Ok(String::from_utf8(buffer)?.into_boxed_str())
    }

    fn read_data_sub_blocks(&mut self) -> Result<Box<[u8]>> {
        let mut block_size = self.read_byte()?;

        // there could be more than one block, but we do know we'll at least have 1 sub-block.
        // allocate capacity to account for it.
        let mut result = Vec::with_capacity(block_size.into());

        // we might have read the block terminator at the end of the while loop, stop right there
        // because we're done.
        while block_size != 0 {
            //println!("trying to read sub_blocks with block size of {:?}", block_size);
            let mut sub_block_buffer = vec![0; block_size.into()];

            self.inner.read(&mut sub_block_buffer)?;
            result.append(&mut sub_block_buffer);

            block_size = self.read_byte()?;
        }

        Ok(result.into_boxed_slice())
    }
}
