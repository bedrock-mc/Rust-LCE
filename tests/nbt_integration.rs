use lce_rust::save::nbt::{
    NbtCompound, NbtError, NbtList, NbtRoot, NbtTag, TagType, read_root_from_bytes,
    write_root_to_bytes,
};

#[test]
fn round_trips_nested_nbt_payload() {
    let position = NbtList::from_elements(vec![NbtTag::Int(12), NbtTag::Int(64), NbtTag::Int(-4)])
        .expect("list should be homogeneous");

    let mut player = NbtCompound::new();
    player.insert("Name", NbtTag::String("Steve".to_string()));
    player.insert("Health", NbtTag::Short(20));
    player.insert("Pos", NbtTag::List(position));

    let mut level = NbtCompound::new();
    level.insert("Seed", NbtTag::Long(1_234_567_890));
    level.insert("Time", NbtTag::Long(6_000));
    level.insert("Player", NbtTag::Compound(player));
    level.insert("Heights", NbtTag::IntArray(vec![64, 65, 66]));
    level.insert("Light", NbtTag::ByteArray(vec![0, 15, -1]));

    let root = NbtRoot::new("Level", level);
    let encoded = write_root_to_bytes(&root).expect("nbt payload should serialize");
    let decoded = read_root_from_bytes(&encoded).expect("nbt payload should deserialize");

    assert_eq!(decoded, root);
}

#[test]
fn parses_and_rewrites_known_compound_bytes() {
    let expected = vec![
        0x0A, 0x00, 0x04, b'D', b'a', b't', b'a', 0x01, 0x00, 0x01, b'b', 0x7F, 0x03, 0x00, 0x01,
        b'i', 0x07, 0x5B, 0xCD, 0x15, 0x08, 0x00, 0x01, b's', 0x00, 0x02, b'o', b'k', 0x00,
    ];

    let root = read_root_from_bytes(&expected).expect("known payload should decode");
    assert_eq!(root.name, "Data");
    assert!(matches!(root.compound.get("b"), Some(NbtTag::Byte(127))));
    assert!(matches!(
        root.compound.get("i"),
        Some(NbtTag::Int(123_456_789))
    ));
    assert!(matches!(
        root.compound.get("s"),
        Some(NbtTag::String(value)) if value == "ok"
    ));

    let encoded = write_root_to_bytes(&root).expect("known payload should encode");
    assert_eq!(encoded, expected);
}

#[test]
fn writes_modified_utf_for_nul_characters() {
    let mut compound = NbtCompound::new();
    compound.insert("s", NbtTag::String("\0".to_string()));

    let root = NbtRoot::new("T", compound);
    let encoded = write_root_to_bytes(&root).expect("payload should serialize");

    let expected = vec![
        0x0A, 0x00, 0x01, b'T', 0x08, 0x00, 0x01, b's', 0x00, 0x02, 0xC0, 0x80, 0x00,
    ];

    assert_eq!(encoded, expected);

    let decoded = read_root_from_bytes(&encoded).expect("payload should deserialize");
    assert!(matches!(
        decoded.compound.get("s"),
        Some(NbtTag::String(value)) if value == "\0"
    ));
}

#[test]
fn rejects_mixed_list_types() {
    let result = NbtList::from_elements(vec![NbtTag::Int(1), NbtTag::Short(2)]);

    assert!(matches!(
        result,
        Err(NbtError::ListTypeMismatch {
            expected: TagType::Int,
            found: TagType::Short
        })
    ));
}

#[test]
fn rejects_non_compound_roots() {
    let int_root_bytes = vec![
        TagType::Int as u8,
        0x00,
        0x00, // empty root name
        0x00,
        0x00,
        0x00,
        0x2A, // int payload
    ];

    let error = read_root_from_bytes(&int_root_bytes).expect_err("non-compound root must fail");
    assert!(matches!(error, NbtError::InvalidRootTag(TagType::Int)));
}
