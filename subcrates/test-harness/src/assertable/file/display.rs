/// Formats the given byte `slice` up to the specified `maximum_displayed_length`.
///
/// If the slice is longer than the maximum length, the "overflowing" bytes are not displayed.
///
/// ```ignore
/// assert_eq!(
///     abbreviate_u8_slice_as_bytes(&[1, 2, 3, 4], 2),
///     "[1, 2, ... (2 remaining)]"
/// );
/// ```
pub(crate) fn abbreviate_u8_slice_as_bytes(
    slice: &[u8],
    maximum_displayed_length: usize,
) -> String {
    if slice.len() <= maximum_displayed_length {
        return format!("{:?}", slice);
    }


    let mut elements = Vec::with_capacity(slice.len().min(maximum_displayed_length));

    for element in slice.iter().take(maximum_displayed_length) {
        elements.push(format!("{}", *element));
    }


    format!(
        "[{}, ... ({} remaining)]",
        elements.join(", "),
        slice.len().saturating_sub(maximum_displayed_length)
    )
}


/// Formats the given byte `slice` as an UTF-8 string, up to the specified `maximum_displayed_chars`.
///
/// If the parsed string is longer than the maximum allowed characters, the rest are not displayed.
///
/// If the slice is not valid UTF-8, the "(not UTF-8)" string is returned.
///
/// ```ignore
/// assert_eq!(
///     abbreviate_u8_slice_as_string(&[72, 69, 76, 76, 79, 32, 87, 79, 82, 76, 68], 7),
///     "HELLO W [4 remaining]"
/// );
/// ```
pub(crate) fn abbreviate_u8_slice_as_string(
    slice: &[u8],
    maximum_displayed_chars: usize,
) -> String {
    let Ok(as_string) = String::from_utf8(slice.to_vec()) else {
        return "(not UTF-8)".to_string();
    };


    format!(
        "{} [{} remaining]",
        as_string
            .chars()
            .take(maximum_displayed_chars)
            .collect::<String>(),
        as_string
            .chars()
            .count()
            .saturating_sub(maximum_displayed_chars)
    )
}
