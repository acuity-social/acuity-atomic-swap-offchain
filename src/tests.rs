//use super::*;
use super::shared::*;

const STR12: &str = "012345678901";
const STR16: &str = "0123456789012345";
const STR32: &str = "01234567890123456789012345678901";
const STR48: &str = "012345678901234567890123456789012345678901234567";

#[test]
fn test_array_to_vec() { // redundant func?
    assert_eq!(array_to_vec(STR16.as_bytes()), STR16.as_bytes().to_vec());
}

#[test]
fn test_vector_as_u8_32_array_offset() { // redundant func?
    let vec:Vec<u8> = STR48.as_bytes().to_vec();
    assert_eq!(vector_as_u8_32_array_offset(&vec, 3), &vec[3..35]);
}

#[test]
#[should_panic]
fn test_vector_as_u8_32_array_offset_panic() {
    vector_as_u8_32_array_offset(&STR32.as_bytes().to_vec(), 3);
}

#[test]
fn test_vector_as_u8_32_array() { // redundant func?
    let vec:Vec<u8> = STR48.as_bytes().to_vec();
    assert_eq!(vector_as_u8_32_array(&vec), vec[..32]);
}

#[test]
#[should_panic]
fn test_vector_as_u8_32_array_panic() {
    vector_as_u8_32_array(&STR16.as_bytes().to_vec());
}

#[test]
fn vector_as_u8_20_array_offset_test() { // redundant func?
    let vec:Vec<u8> = STR32.as_bytes().to_vec();
    assert_eq!(vector_as_u8_20_array_offset(&vec, 3), &vec[3..23]);
}

#[test]
#[should_panic]
fn vector_as_u8_20_array_offset_panic() {
    vector_as_u8_20_array_offset(&STR16.as_bytes().to_vec(), 0);
}

#[test]
fn vector_as_u8_16_array_offset_test() { // redundant func?
    let vec:Vec<u8> = STR32.as_bytes().to_vec();
    assert_eq!(vector_as_u8_16_array_offset(&vec, 3), &vec[3..19]);
}

#[test]
#[should_panic]
fn vector_as_u8_16_array_offset_panic() {
    vector_as_u8_16_array_offset(&STR16.as_bytes().to_vec(), 3);
}

#[test]
fn vector_as_u8_16_array_test() { // redundant func?
    let vec:Vec<u8> = STR16.as_bytes().to_vec();
    assert_eq!(vector_as_u8_16_array(&vec), vec[..16]);
}

#[test]
#[should_panic]
fn vector_as_u8_16_array_panic() {
    vector_as_u8_16_array(&STR12.as_bytes().to_vec());
}
