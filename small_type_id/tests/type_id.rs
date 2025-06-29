use small_type_id::{HasTypeId, TypeId};

#[derive(small_type_id::HasTypeId)]
struct A;

#[derive(small_type_id::HasTypeId)]
struct B;

#[test]
fn serialize_deserialize() {
    let bytes_a = A::TYPE_ID.to_bytes();
    let bytes_b = B::TYPE_ID.to_bytes();
    assert_ne!(bytes_a, bytes_b);
    unsafe {
        assert_eq!(TypeId::from_bytes(bytes_a), Ok(A::TYPE_ID));
        assert_eq!(TypeId::from_bytes(bytes_b), Ok(B::TYPE_ID));
    }
}

#[test]
fn as_usize() {
    assert_eq!(A::TYPE_ID.as_u32() as usize, A::TYPE_ID.as_usize());
    assert_eq!(B::TYPE_ID.as_u32() as usize, B::TYPE_ID.as_usize());
    assert_ne!(B::TYPE_ID.as_u32() as usize, A::TYPE_ID.as_usize());
    assert_ne!(A::TYPE_ID.as_u32() as usize, B::TYPE_ID.as_usize());
    assert_ne!(A::TYPE_ID.as_u32(), B::TYPE_ID.as_u32());
}
