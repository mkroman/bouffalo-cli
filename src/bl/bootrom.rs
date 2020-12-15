use num_enum::{FromPrimitive, IntoPrimitive};
use thiserror::Error;

/// Indicates an error received from the BootROM
#[repr(u16)]
#[derive(Error, Debug, IntoPrimitive, FromPrimitive)]
pub enum Error {
    #[error("BFLB_BOOTROM_SUCCESS")]
    #[num_enum(default)]
    NoError = 0x00,

    #[error("Could not initialize the flash")]
    FlashInitError = 0x0001,
    #[error("Flash parameter issue")]
    FlashParamError,
    #[error("There was an issue when trying to erase the flash")]
    FlashEraseError,
    #[error("Flash write parameter issue")]
    FlashWriteParamError,
    #[error("Flash write address issue")]
    FlashWriteAddrError,
    #[error("Flash write error")]
    FlashWriteError,
    #[error("Flash boot parameter issue")]
    FlashBootParam,

    // Command errors
    #[error("Command id error - possibly an unknown command?")]
    CommandIdError = 0x0101,
    #[error("There was an issue with the length of the command or its parameters")]
    CommandLengthError,
    #[error("There was a CRC checksum error within the command")]
    CommandCrcError,
    #[error("There was a problem with the sequence of commands - this usually means that the command was unexpected in this order")]
    CommandSeqError,

    // Boot header errors
    #[error("Boot header length mismatch")]
    BootHeaderLengthMismatch = 0x0201,
    #[error("Boot header has not been loaded")]
    BootHeaderNotLoaded,
    #[error("The boot header magic value is incorrect")]
    BootHeaderMagicError,
    #[error("The boot header crc32 checksum does not match the boot header")]
    BootHeaderChecksumError,
    #[error("Efuse for encryption has been set, but the bootheader is missing an encryption type")]
    BootHeaderEncryptionMismatch,
    #[error("Efuse for signature validation has been set, but the bootheader is missing a signature type")]
    BootHeaderSignatureMismatch,

    #[error("BFLB_BOOTROM_IMG_SEGMENT_CNT_ERROR")]
    ImageSegmentCountError,
    #[error("BFLB_BOOTROM_IMG_AES_IV_LEN_ERROR")]
    ImageAesIvLengthError,
    #[error("BFLB_BOOTROM_IMG_AES_IV_CRC_ERROR")]
    ImageAesIvChecksumError,
    #[error("BFLB_BOOTROM_IMG_PK_LEN_ERROR")]
    ImagePkLengthError = 0x020a,
    #[error("BFLB_BOOTROM_IMG_PK_CRC_ERROR")]
    ImagePkChecksumError = 0x020b,
    #[error("BFLB_BOOTROM_IMG_PK_HASH_ERROR")]
    ImagePkHashError = 0x020c,
    #[error("BFLB_BOOTROM_IMG_SIGNATURE_LEN_ERROR")]
    ImageSignatureLengthError = 0x020d,
    #[error("BFLB_BOOTROM_IMG_SIGNATURE_CRC_ERROR")]
    ImageSignatureChecksumError = 0x020e,
    #[error("BFLB_BOOTROM_IMG_SECTIONHEADER_LEN_ERROR")]
    ImageSectionHeaderLengthError = 0x020f,
    #[error("BFLB_BOOTROM_IMG_SECTIONHEADER_CRC_ERROR")]
    ImageSectionHeaderChecksumError = 0x0210,
    #[error("BFLB_BOOTROM_IMG_SECTIONHEADER_DST_ERROR")]
    ImageSectionHeaderDstError = 0x0211,
    #[error("BFLB_BOOTROM_IMG_SECTIONDATA_LEN_ERROR")]
    ImageSectionDataLengthError = 0x0212,
    #[error("BFLB_BOOTROM_IMG_SECTIONDATA_DEC_ERROR")]
    ImageSectionDataDecError = 0x0213,

    #[error("BFLB_BOOTROM_IMG_SECTIONDATA_TLEN_ERROR")]
    ImageSectioNDataTlenError = 0x0214,

    #[error("BFLB_BOOTROM_IMG_SECTIONDATA_CRC_ERROR")]
    ImageSectionDataChecksumError = 0x0215,

    #[error("BFLB_BOOTROM_IMG_HALFBAKED_ERROR")]
    ImageHalfBakedError = 0x0216,

    #[error("BFLB_BOOTROM_IMG_HASH_ERROR")]
    ImageHashError = 0x0217,

    #[error("BFLB_BOOTROM_IMG_SIGN_PARSE_ERROR")]
    ImageSignatureParseError = 0x0218,

    #[error("BFLB_BOOTROM_IMG_SIGN_ERROR")]
    ImageSignatureError = 0x0219,

    #[error("BFLB_BOOTROM_IMG_DEC_ERROR")]
    ImageDecError = 0x021a,

    #[error("BFLB_BOOTROM_IMG_ALL_INVALID_ERROR")]
    ImageAllInvalidError = 0x021b,

    #[error("BFLB_BOOTROM_IF_RATE_LEN_ERROR")]
    IfRateLengthError = 0x0301,

    #[error("BFLB_BOOTROM_IF_RATE_PARA_ERROR")]
    IfRateParamError = 0x0302,

    #[error("BFLB_BOOTROM_IF_PASSWORDERROR")]
    IfPasswordError = 0x0303,

    #[error("BFLB_BOOTROM_IF_PASSWORDCLOSE")]
    IfPasswordClose = 0x0304,

    #[error("BFLB_BOOTROM_PLL_ERROR")]
    PllError = 0xfffc,

    #[error("BFLB_BOOTROM_INVASION_ERROR")]
    InvasionError = 0xfffd,

    #[error("BFLB_BOOTROM_POLLING")]
    Polling = 0xfffe,

    #[error("BFLB_BOOTROM_FAIL")]
    Fail = 0xffff,
}
