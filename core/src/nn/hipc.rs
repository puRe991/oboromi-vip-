#[repr(C)]
pub struct HeaderData {
    pub header: [u32; 2],
}

#[repr(C)]
pub struct MapData {
    pub data: [u32; 3],
}

#[repr(C)]
pub struct PointerData {
    pub data: [u32; 2],
}

#[repr(C)]
pub struct ReceiveListData {
    pub data: [u32; 2],
}

#[repr(u32)]
pub enum Result {
    Success = 0,
    Failure = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    TooShort { expected: usize, actual: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub tag: u32,
    pub ptrs_count: u32,
    pub send_count: u32,
    pub recv_count: u32,
    pub xchg_count: u32,
    pub raw_count: u32,
    pub recv_list_count: u32,
    pub recv_list_offs: u32,
    pub special_count: u32,
}

fn read_u32_le(data: &[u8], offset: usize) -> std::result::Result<u32, ParseError> {
    let bytes = data.get(offset..offset + 4).ok_or(ParseError::TooShort {
        expected: offset + 4,
        actual: data.len(),
    })?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

pub fn parse_header(data: &[u8]) -> std::result::Result<Header, ParseError> {
    let hdr0 = read_u32_le(data, 0)?;
    let hdr1 = read_u32_le(data, 4)?;

    Ok(Header {
        tag: hdr0 & 0xffff,
        ptrs_count: (hdr0 >> 16) & 0xf,
        send_count: (hdr0 >> 20) & 0xf,
        recv_count: (hdr0 >> 24) & 0xf,
        xchg_count: (hdr0 >> 28) & 0xf,
        raw_count: hdr1 & ((1 << 10) - 1),
        recv_list_count: (hdr1 >> 10) & 0xf,
        recv_list_offs: (hdr1 >> 14) & 0x3ff,
        special_count: (hdr1 >> 31) & 0x1,
    })
}

pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: FnOnce(Header) -> Result,
{
    match parse_header(data) {
        Ok(header) => f(header),
        Err(_) => Result::Failure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_headers_without_ub() {
        assert_eq!(
            parse_header(&[0; 7]),
            Err(ParseError::TooShort {
                expected: 8,
                actual: 7
            })
        );
        assert!(matches!(
            invoke_method(&[0; 3], |_| Result::Success),
            Result::Failure
        ));
    }

    #[test]
    fn parses_header_fields_deterministically() {
        let hdr0 = 0x4321u32 | (2 << 16) | (3 << 20) | (4 << 24) | (5 << 28);
        let hdr1 = 7u32 | (8 << 10) | (9 << 14) | (1 << 31);
        let mut data = Vec::new();
        data.extend_from_slice(&hdr0.to_le_bytes());
        data.extend_from_slice(&hdr1.to_le_bytes());

        let header = parse_header(&data).expect("valid HIPC header");
        assert_eq!(header.tag, 0x4321);
        assert_eq!(header.ptrs_count, 2);
        assert_eq!(header.send_count, 3);
        assert_eq!(header.recv_count, 4);
        assert_eq!(header.xchg_count, 5);
        assert_eq!(header.raw_count, 7);
        assert_eq!(header.recv_list_count, 8);
        assert_eq!(header.recv_list_offs, 9);
        assert_eq!(header.special_count, 1);
    }
}
