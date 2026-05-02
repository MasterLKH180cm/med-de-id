use std::collections::BTreeMap;

use mdid_domain::{
    ConservativeMediaCandidate, ConservativeMediaFormat, ConservativeMediaRef,
    ConservativeMediaScanStatus, ConservativeMediaSummary,
};

const CONSERVATIVE_METADATA_CONFIDENCE: f32 = 0.35;
const METADATA_IDENTIFIER_PHI_TYPE: &str = "metadata_identifier";
const FCS_HEADER_LEN: usize = 58;
const FCS_MAX_HEADER_OFFSET: usize = 99_999_999;

fn classify_conservative_media_phi_type(
    format: ConservativeMediaFormat,
    metadata_key: &str,
) -> &'static str {
    if format != ConservativeMediaFormat::Fcs {
        return METADATA_IDENTIFIER_PHI_TYPE;
    }

    match metadata_key.trim().to_ascii_uppercase().as_str() {
        "$FIL" | "FILENAME" | "FILE" => "fcs_filename_identifier",
        "$SMNO" | "SMNO" | "SAMPLE_ID" | "SAMPLEID" | "SPECIMEN_ID" => "fcs_sample_identifier",
        "$SRC" | "SRC" | "SOURCE" | "SPECIMEN_SOURCE" => "fcs_source_identifier",
        "$OP" | "OP" | "OPERATOR" | "CREATOR" => "fcs_operator_identifier",
        "$DATE" | "DATE" | "COLLECTION_DATE" | "ACQUISITION_DATE" => "fcs_collection_date",
        _ => METADATA_IDENTIFIER_PHI_TYPE,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConservativeMediaAdapterError {
    EmptyArtifactLabel,
    UnsupportedFcsVersion,
    NonFcsArtifact,
    InvalidFcsHeader,
    InvalidFcsTextSegment,
}

impl std::fmt::Display for ConservativeMediaAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArtifactLabel => f.write_str("artifact label must not be empty"),
            Self::UnsupportedFcsVersion => f.write_str("unsupported FCS version"),
            Self::NonFcsArtifact => f.write_str("artifact is not FCS"),
            Self::InvalidFcsHeader => f.write_str("invalid FCS header"),
            Self::InvalidFcsTextSegment => f.write_str("invalid FCS TEXT segment"),
        }
    }
}

impl std::error::Error for ConservativeMediaAdapterError {}

#[derive(Clone, PartialEq, Eq)]
pub struct ConservativeMediaMetadataEntry {
    pub key: String,
    pub value: String,
}

impl std::fmt::Debug for ConservativeMediaMetadataEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaMetadataEntry")
            .field("key", &self.key)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct ConservativeMediaInput {
    pub artifact_label: String,
    pub format: ConservativeMediaFormat,
    pub metadata: Vec<ConservativeMediaMetadataEntry>,
    pub requires_visual_review: bool,
    pub unsupported_payload: bool,
}

impl std::fmt::Debug for ConservativeMediaInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConservativeMediaInput")
            .field("artifact_label", &"<redacted>")
            .field("format", &self.format)
            .field("metadata", &self.metadata)
            .field("requires_visual_review", &self.requires_visual_review)
            .field("unsupported_payload", &self.unsupported_payload)
            .finish()
    }
}

#[derive(Clone)]
pub struct ExtractedConservativeMediaData {
    pub candidates: Vec<ConservativeMediaCandidate>,
    pub summary: ConservativeMediaSummary,
}

impl std::fmt::Debug for ExtractedConservativeMediaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractedConservativeMediaData")
            .field(
                "candidates",
                &RedactedConservativeMediaCandidates(&self.candidates),
            )
            .field("summary", &self.summary)
            .finish()
    }
}

struct RedactedConservativeMediaCandidates<'a>(&'a [ConservativeMediaCandidate]);

impl std::fmt::Debug for RedactedConservativeMediaCandidates<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for candidate in self.0 {
            list.entry(&RedactedConservativeMediaCandidate(candidate));
        }
        list.finish()
    }
}

struct RedactedConservativeMediaCandidate<'a>(&'a ConservativeMediaCandidate);

impl std::fmt::Debug for RedactedConservativeMediaCandidate<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let candidate = self.0;
        f.debug_struct("ConservativeMediaCandidate")
            .field("field_ref", &"<redacted>")
            .field("format", &candidate.format)
            .field("phi_type", &candidate.phi_type)
            .field("source_value", &"<redacted>")
            .field("confidence", &candidate.confidence)
            .field("status", &candidate.status)
            .finish()
    }
}

pub struct ConservativeMediaAdapter;

#[derive(Clone, PartialEq, Eq)]
pub struct FcsTextRewriteRequest {
    pub replacements: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FcsTextRewriteSummary {
    pub replacement_count: usize,
    pub input_text_byte_len: usize,
    pub output_text_byte_len: usize,
    pub input_byte_len: usize,
    pub output_byte_len: usize,
    pub rewritten_key_count: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FcsTextRewriteOutput {
    pub bytes: Vec<u8>,
    pub summary: FcsTextRewriteSummary,
}

impl std::fmt::Debug for FcsTextRewriteOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FcsTextRewriteOutput")
            .field("bytes", &"<redacted>")
            .field("summary", &self.summary)
            .finish()
    }
}

impl ConservativeMediaAdapter {
    pub fn rewrite_fcs_text_segment(
        bytes: &[u8],
        request: FcsTextRewriteRequest,
    ) -> Result<FcsTextRewriteOutput, ConservativeMediaAdapterError> {
        if bytes.len() < 58 {
            return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
        }
        if &bytes[0..3] != b"FCS" {
            return Err(ConservativeMediaAdapterError::NonFcsArtifact);
        }
        if &bytes[3..5] != b"3." {
            return Err(ConservativeMediaAdapterError::UnsupportedFcsVersion);
        }
        let text_start = parse_fcs_offset(&bytes[10..18])?;
        let text_end = parse_fcs_offset(&bytes[18..26])?;
        if text_start < FCS_HEADER_LEN
            || text_start >= bytes.len()
            || text_end >= bytes.len()
            || text_start > text_end
        {
            return Err(ConservativeMediaAdapterError::InvalidFcsTextSegment);
        }
        let old_text_end = text_end;
        let text = &bytes[text_start..=text_end];
        if text.is_empty() {
            return Err(ConservativeMediaAdapterError::InvalidFcsTextSegment);
        }
        let delimiter = text[0];
        if delimiter == 0 || !delimiter.is_ascii() {
            return Err(ConservativeMediaAdapterError::InvalidFcsTextSegment);
        }
        let parts = parse_fcs_text_parts(&text[1..], delimiter)?;
        if parts.len() % 2 != 0 {
            return Err(ConservativeMediaAdapterError::InvalidFcsTextSegment);
        }
        let mut output_text = Vec::with_capacity(text.len());
        output_text.push(delimiter);
        let mut rewritten_key_count = 0;
        for pair in parts.chunks(2) {
            let key = std::str::from_utf8(&pair[0])
                .map_err(|_| ConservativeMediaAdapterError::InvalidFcsTextSegment)?;
            append_fcs_escaped(&mut output_text, &pair[0], delimiter);
            output_text.push(delimiter);
            if let Some(replacement) = request.replacements.get(key) {
                append_fcs_escaped(&mut output_text, replacement.as_bytes(), delimiter);
                rewritten_key_count += 1;
            } else {
                append_fcs_escaped(&mut output_text, &pair[1], delimiter);
            }
            output_text.push(delimiter);
        }
        let new_text_end = text_start
            .checked_add(output_text.len())
            .and_then(|value| value.checked_sub(1))
            .ok_or(ConservativeMediaAdapterError::InvalidFcsHeader)?;
        ensure_fcs_header_offset(new_text_end)?;
        let output_len = bytes
            .len()
            .checked_sub(text.len())
            .and_then(|value| value.checked_add(output_text.len()))
            .ok_or(ConservativeMediaAdapterError::InvalidFcsHeader)?;
        let mut out = Vec::with_capacity(output_len);
        out.extend_from_slice(&bytes[..text_start]);
        out.extend_from_slice(&output_text);
        out.extend_from_slice(&bytes[text_end + 1..]);
        write_fcs_header_offset(&mut out[18..26], new_text_end)?;
        shift_fcs_segment_offsets(
            &mut out,
            26..34,
            34..42,
            old_text_end,
            text.len(),
            output_text.len(),
        )?;
        shift_fcs_segment_offsets(
            &mut out,
            42..50,
            50..58,
            old_text_end,
            text.len(),
            output_text.len(),
        )?;
        let summary = FcsTextRewriteSummary {
            replacement_count: rewritten_key_count,
            input_text_byte_len: text.len(),
            output_text_byte_len: output_text.len(),
            input_byte_len: bytes.len(),
            output_byte_len: out.len(),
            rewritten_key_count,
        };
        Ok(FcsTextRewriteOutput {
            bytes: out,
            summary,
        })
    }

    pub fn extract_metadata(
        input: ConservativeMediaInput,
    ) -> Result<ExtractedConservativeMediaData, ConservativeMediaAdapterError> {
        if input.artifact_label.trim().is_empty() {
            return Err(ConservativeMediaAdapterError::EmptyArtifactLabel);
        }

        let mut summary = ConservativeMediaSummary {
            total_items: 1,
            metadata_only_items: 0,
            visual_review_required_items: 0,
            unsupported_items: 0,
            review_required_candidates: 0,
        };

        if input.unsupported_payload {
            summary.unsupported_items = 1;
            return Ok(ExtractedConservativeMediaData {
                candidates: Vec::new(),
                summary,
            });
        }

        let status = if input.requires_visual_review {
            summary.visual_review_required_items = 1;
            ConservativeMediaScanStatus::OcrOrVisualReviewRequired
        } else {
            summary.metadata_only_items = 1;
            ConservativeMediaScanStatus::MetadataOnly
        };

        let candidates = input
            .metadata
            .into_iter()
            .filter(|entry| !entry.key.trim().is_empty() && !entry.value.trim().is_empty())
            .map(|entry| {
                let phi_type =
                    classify_conservative_media_phi_type(input.format, entry.key.as_str());

                ConservativeMediaCandidate {
                    field_ref: ConservativeMediaRef {
                        artifact_label: input.artifact_label.clone(),
                        metadata_key: entry.key,
                    },
                    format: input.format,
                    phi_type: phi_type.to_string(),
                    source_value: entry.value,
                    confidence: CONSERVATIVE_METADATA_CONFIDENCE,
                    status,
                }
            })
            .collect::<Vec<_>>();

        summary.review_required_candidates = candidates.len();

        Ok(ExtractedConservativeMediaData {
            candidates,
            summary,
        })
    }
}

fn parse_fcs_offset(bytes: &[u8]) -> Result<usize, ConservativeMediaAdapterError> {
    parse_optional_fcs_offset(bytes)?.ok_or(ConservativeMediaAdapterError::InvalidFcsHeader)
}

fn parse_optional_fcs_offset(bytes: &[u8]) -> Result<Option<usize>, ConservativeMediaAdapterError> {
    if bytes.len() != 8 {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| ConservativeMediaAdapterError::InvalidFcsHeader)?;
    if !text.chars().all(|c| c == ' ' || c.is_ascii_digit()) {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| ConservativeMediaAdapterError::InvalidFcsHeader)
}

fn ensure_fcs_header_offset(offset: usize) -> Result<(), ConservativeMediaAdapterError> {
    if offset > FCS_MAX_HEADER_OFFSET {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    Ok(())
}

fn write_fcs_header_offset(
    target: &mut [u8],
    offset: usize,
) -> Result<(), ConservativeMediaAdapterError> {
    ensure_fcs_header_offset(offset)?;
    if target.len() != 8 {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    let formatted = format!("{offset:>8}");
    if formatted.len() != 8 {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    target.copy_from_slice(formatted.as_bytes());
    Ok(())
}

fn parse_fcs_text_parts(
    bytes: &[u8],
    delimiter: u8,
) -> Result<Vec<Vec<u8>>, ConservativeMediaAdapterError> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == delimiter {
            if bytes.get(index + 1) == Some(&delimiter) {
                current.push(delimiter);
                index += 2;
            } else {
                parts.push(current);
                current = Vec::new();
                index += 1;
            }
        } else {
            current.push(byte);
            index += 1;
        }
    }
    if !current.is_empty() {
        return Err(ConservativeMediaAdapterError::InvalidFcsTextSegment);
    }
    Ok(parts)
}

fn append_fcs_escaped(target: &mut Vec<u8>, value: &[u8], delimiter: u8) {
    for byte in value {
        target.push(*byte);
        if *byte == delimiter {
            target.push(delimiter);
        }
    }
}

fn shift_fcs_segment_offsets(
    header: &mut [u8],
    begin_range: std::ops::Range<usize>,
    end_range: std::ops::Range<usize>,
    old_text_end: usize,
    old_text_len: usize,
    new_text_len: usize,
) -> Result<(), ConservativeMediaAdapterError> {
    let Some(begin) = parse_optional_fcs_offset(&header[begin_range.clone()])? else {
        return Ok(());
    };
    let Some(end) = parse_optional_fcs_offset(&header[end_range.clone()])? else {
        return Ok(());
    };
    if begin == 0 && end == 0 {
        return Ok(());
    }
    if begin <= old_text_end || end < begin {
        return Err(ConservativeMediaAdapterError::InvalidFcsHeader);
    }
    let shifted_begin = shift_offset(begin, old_text_len, new_text_len)?;
    let shifted_end = shift_offset(end, old_text_len, new_text_len)?;
    write_fcs_header_offset(&mut header[begin_range], shifted_begin)?;
    write_fcs_header_offset(&mut header[end_range], shifted_end)?;
    Ok(())
}

fn shift_offset(
    offset: usize,
    old_text_len: usize,
    new_text_len: usize,
) -> Result<usize, ConservativeMediaAdapterError> {
    let shifted = if new_text_len >= old_text_len {
        offset.checked_add(new_text_len - old_text_len)
    } else {
        offset.checked_sub(old_text_len - new_text_len)
    }
    .ok_or(ConservativeMediaAdapterError::InvalidFcsHeader)?;
    ensure_fcs_header_offset(shifted)?;
    Ok(shifted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fcs_header_offset_writer_fails_closed_on_overflow_without_panic() {
        let mut target = *b"00000000";
        let err = write_fcs_header_offset(&mut target, FCS_MAX_HEADER_OFFSET + 1).unwrap_err();

        assert_eq!(err, ConservativeMediaAdapterError::InvalidFcsHeader);
        assert_eq!(&target, b"00000000");
    }
}
