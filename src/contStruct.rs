
use std::fs;
use std::mem::size_of;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Feed file path builder
// ─────────────────────────────────────────────────────────────────────────────

/// Market segment supported by the NSE feed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    NseCm,
    NseFo,
}

impl Segment {
    /// The subdirectory name under the base path (e.g. "NSE_CM").
    pub fn folder_name(&self) -> &'static str {
        match self {
            Segment::NseCm => "NSE_CM",
            Segment::NseFo => "NSE_FO",
        }
    }

    /// The short suffix used inside the filename (e.g. "CM").
    pub fn short_name(&self) -> &'static str {
        match self {
            Segment::NseCm => "CM",
            Segment::NseFo => "FO",
        }
    }

    /// Parse from user input — accepts "NSE_CM", "CM", "NSE_FO", or "FO" (case-insensitive).
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_uppercase().as_str() {
            "NSE_CM" | "CM" => Ok(Segment::NseCm),
            "NSE_FO" | "FO" => Ok(Segment::NseFo),
            other => Err(format!(
                "unknown segment '{other}' — expected one of: NSE_CM, CM, NSE_FO, FO"
            )),
        }
    }
}

/// Builds and validates NSE binary feed file paths.
///
/// File naming convention:
///   `/nas/50.30/{SEGMENT}/Feed_{short}_StreamID_{stream_id}_{DD}_{MM}_{YYYY}.bin`
///
/// Example:
///   `/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin`
pub struct FeedFilePath;

impl FeedFilePath {
    /// Build the full file path from individual components.
    ///
    /// # Arguments
    /// * `segment`   — "NSE_CM", "CM", "NSE_FO", or "FO" (case-insensitive)
    /// * `stream_id` — positive integer stream ID  
    /// * `day`       — 1–31
    /// * `month`     — 1–12
    /// * `year`      — 2000–2100
    /// * `base_path` — optional override; defaults to "/nas/50.30"
    pub fn build(
        segment: &str,
        stream_id: u32,
        day: u32,
        month: u32,
        year: u32,
        base_path: Option<&str>,
    ) -> Result<String, String> {
        let seg = Segment::from_str(segment)?;

        if stream_id == 0 {
            return Err("stream_id must be > 0".to_string());
        }
        if !(1..=31).contains(&day) {
            return Err(format!("invalid day {day} — must be 1–31"));
        }
        if !(1..=12).contains(&month) {
            return Err(format!("invalid month {month} — must be 1–12"));
        }
        if !(2000..=2100).contains(&year) {
            return Err(format!("invalid year {year} — must be 2000–2100"));
        }

        let root = base_path.unwrap_or("/nas/50.30");
        let path = format!(
            "{}/{}/Feed_{}_StreamID_{}_{:02}_{:02}_{:04}.bin",
            root.trim_end_matches('/'),
            seg.folder_name(),
            seg.short_name(),
            stream_id,
            day,
            month,
            year,
        );

        Ok(path)
    }

    /// Build path and also verify the file exists on disk.
    pub fn build_and_verify(
        segment: &str,
        stream_id: u32,
        day: u32,
        month: u32,
        year: u32,
        base_path: Option<&str>,
    ) -> Result<String, String> {
        let path = Self::build(segment, stream_id, day, month, year, base_path)?;
        if !std::path::Path::new(&path).exists() {
            return Err(format!("file not found: {path}"));
        }
        Ok(path)
    }
}

#[cfg(test)]
mod path_builder_tests {
    use super::*;

    #[test]
    fn test_build_nse_cm() {
        let p = FeedFilePath::build("NSE_CM", 2, 29, 12, 2025, None).unwrap();
        assert_eq!(p, "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin");
    }

    #[test]
    fn test_build_nse_fo_short_name() {
        let p = FeedFilePath::build("FO", 1, 1, 1, 2024, None).unwrap();
        assert_eq!(p, "/nas/50.30/NSE_FO/Feed_FO_StreamID_1_01_01_2024.bin");
    }

    #[test]
    fn test_custom_base_path() {
        let p = FeedFilePath::build("CM", 3, 5, 6, 2026, Some("/data/nse")).unwrap();
        assert_eq!(p, "/data/nse/NSE_CM/Feed_CM_StreamID_3_05_06_2026.bin");
    }

    #[test]
    fn test_invalid_segment() {
        assert!(FeedFilePath::build("INVALID", 1, 1, 1, 2025, None).is_err());
    }

    #[test]
    fn test_invalid_stream_id_zero() {
        assert!(FeedFilePath::build("CM", 0, 1, 1, 2025, None).is_err());
    }

    #[test]
    fn test_invalid_date() {
        assert!(FeedFilePath::build("CM", 1, 32, 1, 2025, None).is_err());
        assert!(FeedFilePath::build("CM", 1, 1, 13, 2025, None).is_err());
        assert!(FeedFilePath::build("CM", 1, 1, 1, 1999, None).is_err());
    }
}


#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Stream_info {
    pub msg_type: u8,
    pub stream_id: u16,
    pub token_number: u32,
    pub instrument: u8,
    pub symbol: [u8; 10],
    pub expiry_date: u32,
    pub strike_price: u32,
    pub option_type: u8,
}
impl Stream_info {
    pub fn from_le_fields(mut self) -> Self {
        self.msg_type = u8::from_le(self.msg_type);
        self.stream_id = u16::from_le(self.stream_id);
        self.token_number = u32::from_le(self.token_number);
        self.instrument = u32::from_le(self.instrument as u32) as u8;
        
        self.expiry_date = u32::from_le(self.expiry_date);
        self.strike_price = u32::from_le(self.strike_price);
        self.option_type  = u32::from_le(self.option_type as u32) as u8;
        
        self
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ContractFileHeader {
    pub neatfo: [u8; 6],
    pub reserved_6: u8,
    pub version_number: [u8; 5],
    pub reserved_12: u8,
}

impl ContractFileHeader {
    #[inline]
    pub fn from_le_fields(self) -> Self {
        self
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StSecEligibilityPerMarket {
    pub security_status: u16,
    pub reserved_2: u8,
    pub eligibility: u8,
    pub reserved_4: [u8; 2],
}

impl StSecEligibilityPerMarket {
    #[inline]
    pub fn from_le_fields(self) -> Self {
        let mut s = self;
        s.security_status = u16::from_le(s.security_status);
        s
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct StockStructure {
    pub token: u32,
    pub reserved_4: u8,
    pub asset_token: u32,
    pub reserved_9: u8,
    pub instrument_name: [u8; 6],
    pub reserved_16: u8,
    pub symbol: [u8; 10],
    pub reserved_27: u8,
    pub series: [u8; 2],
    pub reserved_30: [u8; 2],
    pub expiry_date: u32,
    pub reserved_36: u8,
    pub strike_price: u32,
    pub reserved_41: u8,
    pub option_type: [u8; 2],
    pub reserved_44: u8,
    pub category: u8,
    pub reserved_46: u8,
    pub ca_level: u16,
    pub reserved_49: u8,
    pub reserved_identifier: u8,
    pub reserved_51: u8,
    pub permitted_to_trade: u16,
    pub reserved_54: u8,
    pub issue_rate: u16,
    pub reserved_57: u8,
    pub st_sec_eligibility_per_market: [StSecEligibilityPerMarket; 4],
    pub issue_start_date: u32,
    pub reserved_68: u8,
    pub interest_payment_date: u32,
    pub reserved_73: u8,
    pub issue_maturity_date: u32,
    pub reserved_78: u8,
    pub margin_percentage: u32,
    pub reserved_83: u8,
    pub minimum_lot_quantity: u32,
    pub reserved_88: u8,
    pub board_lot_quantity: u32,
    pub reserved_93: u8,
    pub tick_size: u32,
    pub reserved_98: u8,
    pub issued_capital: f64,
    pub reserved_107: u8,
    pub freeze_quantity: u32,
    pub reserved_112: u8,
    pub warning_quantity: u32,
    pub reserved_117: u8,
    pub listing_date: u32,
    pub reserved_122: u8,
    pub expulsion_date: u32,
    pub reserved_127: u8,
    pub readmission_date: u32,
    pub reserved_132: u8,
    pub record_date: u32,
    pub reserved_137: u8,
    pub no_delivery_start_date: u32,
    pub reserved_142: u8,
    pub no_delivery_end_date: u32,
    pub reserved_147: u8,
    pub low_price_range: u32,
    pub reserved_152: u8,
    pub high_price_range: u32,
    pub reserved_157: u8,
    pub ex_date: u32,
    pub reserved_162: u8,
    pub book_closure_start_date: u32,
    pub reserved_167: u8,
    pub book_closure_end_date: u32,
    pub reserved_172: u8,
    pub local_ldb_update_date_time: u32,
    pub reserved_177: u8,
    pub exercise_start_date: u32,
    pub reserved_182: u8,
    pub exercise_end_date: u32,
    pub reserved_187: u8,
    pub ticker_selection: u16,
    pub reserved_190: u8,
    pub old_token_number: u32,
    pub reserved_195: u8,
    pub credit_rating: [u8; 12],
    pub reserved_208: u8,
    pub name: [u8; 25],
    pub reserved_234: u8,
    pub egmagm: u8,
    pub reserved_236: u8,
    pub interest_dividend: u8,
    pub reserved_238: u8,
    pub rights_bonus: u8,
    pub reserved_240: u8,
    pub mfaon: u8,
    pub reserved_242: u8,
    pub remarks: [u8; 24],
    pub reserved_267: u8,
    pub ex_style: u8,
    pub reserved_269: u8,
    pub ex_allowed: u8,
    pub reserved_271: u8,
    pub ex_rejection_allowed: u8,
    pub reserved_273: u8,
    pub pl_allowed: u8,
    pub reserved_275: u8,
    pub settlement_indicator: u8,
    pub reserved_277: u8,
    pub is_corporate_adjusted: u8,
    pub reserved_279: u8,
    pub symbol_for_asset: [u8; 10],
    pub reserved_290: u8,
    pub instrument_of_asset: [u8; 6],
    pub reserved_297: u8,
    pub base_price: u32,
    pub reserved_302: u8,
    pub delete_flag: u8,
}

impl StockStructure {
    #[inline]
    pub fn from_le_fields(mut self) -> Self {
        self.token = u32::from_le(self.token);
        self.asset_token = u32::from_le(self.asset_token);
        self.expiry_date = u32::from_le(self.expiry_date);
        self.strike_price = u32::from_le(self.strike_price);
        self.ca_level = u16::from_le(self.ca_level);
        self.permitted_to_trade = u16::from_le(self.permitted_to_trade);
        self.issue_rate = u16::from_le(self.issue_rate);
        for item in &mut self.st_sec_eligibility_per_market {
            *item = item.from_le_fields();
        }
        self.issue_start_date = u32::from_le(self.issue_start_date);
        self.interest_payment_date = u32::from_le(self.interest_payment_date);
        self.issue_maturity_date = u32::from_le(self.issue_maturity_date);
        self.margin_percentage = u32::from_le(self.margin_percentage);
        self.minimum_lot_quantity = u32::from_le(self.minimum_lot_quantity);
        self.board_lot_quantity = u32::from_le(self.board_lot_quantity);
        self.tick_size = u32::from_le(self.tick_size);
        self.issued_capital = f64::from_bits(u64::from_le(self.issued_capital.to_bits()));
        self.freeze_quantity = u32::from_le(self.freeze_quantity);
        self.warning_quantity = u32::from_le(self.warning_quantity);
        self.listing_date = u32::from_le(self.listing_date);
        self.expulsion_date = u32::from_le(self.expulsion_date);
        self.readmission_date = u32::from_le(self.readmission_date);
        self.record_date = u32::from_le(self.record_date);
        self.no_delivery_start_date = u32::from_le(self.no_delivery_start_date);
        self.no_delivery_end_date = u32::from_le(self.no_delivery_end_date);
        self.low_price_range = u32::from_le(self.low_price_range);
        self.high_price_range = u32::from_le(self.high_price_range);
        self.ex_date = u32::from_le(self.ex_date);
        self.book_closure_start_date = u32::from_le(self.book_closure_start_date);
        self.book_closure_end_date = u32::from_le(self.book_closure_end_date);
        self.local_ldb_update_date_time = u32::from_le(self.local_ldb_update_date_time);
        self.exercise_start_date = u32::from_le(self.exercise_start_date);
        self.exercise_end_date = u32::from_le(self.exercise_end_date);
        self.ticker_selection = u16::from_le(self.ticker_selection);
        self.old_token_number = u32::from_le(self.old_token_number);
        self.base_price = u32::from_le(self.base_price);
        self
    }

    pub fn token_val(&self) -> u32 {
        unsafe { 
            let p = std::ptr::addr_of!(self.token) as *const u32; 
            u32::from_le(std::ptr::read_unaligned(p))
        }
    }
    pub fn asset_token_val(&self) -> u32 {
        unsafe { 
            let p = std::ptr::addr_of!(self.asset_token) as *const u32; 
            u32::from_le(std::ptr::read_unaligned(p))
        }
    }
    pub fn strike_price_val(&self) -> u32 {
        unsafe { 
            let p = std::ptr::addr_of!(self.strike_price) as *const u32; 
            u32::from_le(std::ptr::read_unaligned(p))
        }
    }
    pub fn base_price_val(&self) -> u32 {
        unsafe { 
            let p = std::ptr::addr_of!(self.base_price) as *const u32; 
            u32::from_le(std::ptr::read_unaligned(p))
        }
    }
}

pub fn load_contract_from_path<P: AsRef<Path>>(path: P) -> Result<(ContractFileHeader, Box<[StockStructure]>), String> {
    let data: Vec<u8> = fs::read(path).map_err(|e| e.to_string())?;

    let header_len = size_of::<ContractFileHeader>();
    let stock_len = size_of::<StockStructure>();

    if data.len() < header_len {
        return Err("file too small to contain CONTRACT_FILE_HEADER".to_string());
    }

    let mut header: ContractFileHeader = unsafe {
        std::ptr::read_unaligned(data[..header_len].as_ptr() as *const _)
    };
    header = header.from_le_fields();

    let mut records = Vec::new();
    let mut offset = header_len;
    while offset + stock_len <= data.len() {
        let mut stock: StockStructure = unsafe {
            std::ptr::read_unaligned(data[offset..offset + stock_len].as_ptr() as *const _)
        };
        stock = stock.from_le_fields();
        records.push(stock);
        offset += stock_len;
    }

    if offset != data.len() {
        let trailing = &data[offset..];
        eprintln!(
            "Warning: trailing {} bytes at end of contract file: {:02X?}",
            trailing.len(),
            trailing
        );
    }

    Ok((header, records.into_boxed_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;

    fn push_fixed(buf: &mut Vec<u8>, src: &[u8], len: usize) {
        let mut tmp = vec![0u8; len];
        tmp[..src.len().min(len)].copy_from_slice(&src[..src.len().min(len)]);
        buf.extend_from_slice(&tmp);
    }

    #[test]
    fn test_load_contract_from_path_one_record() {
        let header_len = std::mem::size_of::<ContractFileHeader>();
        let stock_len = std::mem::size_of::<StockStructure>();
        let total_len = header_len + stock_len;

        let mut data = Vec::with_capacity(total_len);

        push_fixed(&mut data, b"NEATFO", 6);
        data.push(0);
        push_fixed(&mut data, b"V001\0", 5);
        data.push(0);

        let mut stock_bytes = vec![0u8; stock_len];

        let token = 0x11223344u32;
        stock_bytes[0..4].copy_from_slice(&token.to_le_bytes());

        let asset_token = 0x55667788u32;
        stock_bytes[5..9].copy_from_slice(&asset_token.to_le_bytes());

        let s: StockStructure = unsafe { std::mem::zeroed() };
        let base = &s as *const _ as usize;
        let off_strike = (std::ptr::addr_of!(s.strike_price) as usize) - base;
        let off_expiry = (std::ptr::addr_of!(s.expiry_date) as usize) - base;
        let off_base_price = (std::ptr::addr_of!(s.base_price) as usize) - base;

        let strike_price = 2500u32;
        let expiry_date = 20240101u32;
        let base_price = 1000u32;

        stock_bytes[off_expiry..off_expiry + 4].copy_from_slice(&expiry_date.to_le_bytes());
        stock_bytes[off_strike..off_strike + 4].copy_from_slice(&strike_price.to_le_bytes());
        stock_bytes[off_base_price..off_base_price + 4].copy_from_slice(&base_price.to_le_bytes());

        data.extend(stock_bytes.iter());

        let mut path = env::temp_dir();
        path.push("test_contract_loader.bin");
        let _ = fs::remove_file(&path);

        let mut file = File::create(&path).expect("create temp file");
        file.write_all(&data).expect("write temp file");
        drop(file);

        let (header, records) = load_contract_from_path(&path).expect("should load OK");
        assert_eq!(&header.neatfo, b"NEATFO");
        assert_eq!(&header.version_number, b"V001\0");
        assert_eq!(records.len(), 1);

        let record = &records[0];

        assert_eq!(record.token_val(), token);
        assert_eq!(record.asset_token_val(), asset_token);
        assert_eq!(record.strike_price_val(), strike_price);
        assert_eq!(record.base_price_val(), base_price);

        let _ = fs::remove_file(&path);
    }
}