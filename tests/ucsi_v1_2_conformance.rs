//! Strict UCSI Revision 1.2 wire-format conformance tests.
//!
//! The `ucsi::v1_2` module advertises UCSI Revision 1.2, but several of its public
//! types expose fields that Revision 1.2 explicitly reserves. Those fields were
//! introduced by later revisions. UCSI 1.2 Section 1.5.2.6 says of reserved fields:
//!
//! > field shall be set to zero by the sender and shall be ignored by the receiver.
//!
//! Each test below pins one Revision 1.2 requirement. A failure means the `v1_2`
//! module can emit a command or status that is not legal Revision 1.2 traffic.
//!
//! These tests are deliberately scoped to Revision 1.2 only. They assert nothing
//! about Revision 3.1, which does define many of these bits; the point is that a
//! module named `v1_2` must not be able to produce them. Where Revision 3.1 is
//! mentioned it is to record a trap for a future revision-aware implementation.
//!
//! Driving everything through the public API is intentional: these are exactly the
//! encodings a downstream consumer of the crate can produce.
#![cfg(feature = "ucsi-v1_2")]

use embedded_usb_pd::ucsi::v1_2::cci::Cci;
use embedded_usb_pd::ucsi::v1_2::lpm::{
    connector_reset, get_alternate_modes, get_error_status, set_ccom, set_power_level, Recipient,
};
use embedded_usb_pd::{type_c, GlobalPortId, PowerRole};

/// UCSI 1.2 Table 4-5 (CONNECTOR_RESET Command) reserves command bits 23 through 63:
///
/// > 23            Reserved           41         Reserved and shall be set to zero.
///
/// Revision 1.2 has no hard-reset/data-reset selector; that distinction arrived later.
/// Bit 23 is the top bit of the connector byte, so no encodable `Args` may set it.
///
/// Polarity trap for any future revision-aware type: UCSI 3.1 Table 6-5 assigns `0` to
/// Hard Reset (the default) and `1` to Data Reset, the opposite sense to `hard_reset`.
#[test]
fn connector_reset_must_not_set_reserved_bit_23() {
    let raw = connector_reset::ArgsRaw::from(connector_reset::Args {
        connector_number: 1,
        hard_reset: true,
    });

    assert_eq!(
        raw.connector & 0x80,
        0,
        "UCSI 1.2 Table 4-5 reserves command bit 23; a v1.2 CONNECTOR_RESET must not set it"
    );
}

/// UCSI 1.2 Table 4-18 (SET_CCOM Command) reserves command bits 26 through 63.
///
/// The `disabled` role bit sits at argument bit 10, i.e. command bit 26, and so is not
/// a Revision 1.2 field.
#[test]
fn set_ccom_must_not_set_reserved_disabled_bit() {
    let raw = set_ccom::ArgBitsRaw::from(set_ccom::Args {
        connector_number: 1,
        rp: true,
        rd: false,
        drp: false,
        disabled: true,
    });

    assert!(
        !raw.disabled(),
        "UCSI 1.2 Table 4-18 reserves command bit 26 (argument bit 10); \
         `disabled` is not a Revision 1.2 field"
    );
}

/// UCSI 1.2 Table 4-48 (SET_POWER_LEVEL Command) reserves command bits 34 through 63:
///
/// > Reserved and shall be set to zero.
///
/// Revision 1.2 SET_POWER_LEVEL carries only a connector number, a power role and a
/// Type-C current. The LSB-control, operating-current and output-voltage fields were
/// introduced later and all land in reserved space.
#[test]
fn set_power_level_must_not_set_reserved_bits_34_and_above() {
    let raw = set_power_level::ArgsRaw::try_from(set_power_level::Args {
        connector_number: 1,
        power_role: PowerRole::Source,
        lsb_control: true,
        max_power: 0,
        type_c_current: set_power_level::Current::Current(type_c::Current::Current3A0),
        operating_current: 1500,
        output_voltage: 5000,
    })
    .expect("argument values are within their field widths");

    // The argument block begins at command bit 16, so command bit 34 is argument bit 18:
    // byte 2, bit 2. Everything from there up is reserved in Revision 1.2.
    let reserved_in_byte2 = raw.bits[2] & !0b0000_0011;
    assert_eq!(
        reserved_in_byte2, 0,
        "UCSI 1.2 Table 4-48 reserves command bits 34 and above; \
         argument byte 2 has {reserved_in_byte2:#04x} set"
    );
    for (i, b) in raw.bits.iter().enumerate().skip(3) {
        assert_eq!(
            *b, 0,
            "UCSI 1.2 Table 4-48 reserves command bits 34 and above; argument byte {i} is {b:#04x}"
        );
    }
}

/// UCSI 1.2 Table 4-45 (GET_ERROR_STATUS Command) reserves the entire argument payload:
///
/// > 16         Reserved                 48           Reserved and shall be set to zero.
///
/// Revision 1.2 GET_ERROR_STATUS is not connector-addressed; the connector field was
/// added later (UCSI 3.1 Table 6-46). Encoding a connector number writes reserved bits.
#[test]
fn get_error_status_payload_is_entirely_reserved() {
    let raw = get_error_status::ArgsRaw::from(3u8);

    assert_eq!(
        raw.connector, 0,
        "UCSI 1.2 Table 4-45 reserves the whole GET_ERROR_STATUS payload; \
         the command is not connector-addressed in Revision 1.2"
    );
}

/// UCSI 1.2 Table 4-47 (Error Information) reserves bits 15 through 13:
///
/// > 15:13     Reserved and shall be set to zero.
///
/// Reverse Current Protection and Set Sink Path Rejected are later-revision error bits
/// (UCSI 3.1 Table 6-48) and must not be emitted by a Revision 1.2 implementation.
#[test]
fn error_information_must_not_set_reserved_bits_13_and_14() {
    let raw = get_error_status::InformationRaw::from(get_error_status::Information {
        reverse_current_protection: true,
        sink_path_rejected: true,
        ..Default::default()
    });

    assert!(
        !raw.reverse_current_protection(),
        "UCSI 1.2 Table 4-47 reserves error bit 13"
    );
    assert!(!raw.sink_path_rejected(), "UCSI 1.2 Table 4-47 reserves error bit 14");
}

/// UCSI 1.2 Table 3-2 (CCI) reserves bit 0 and bits 24 through 16:
///
/// > 0             Reserved             1          Reserved and shall be set to zero.
/// > 16       Reserved            9          Reserved and shall be set to zero.
///
/// End of Message, Vendor Defined Message, Security Request and Firmware Update Request
/// are all later-revision CCI indicators (UCSI 3.1 Table 4-3).
#[test]
fn cci_must_not_set_reserved_bit_0_or_bits_16_through_24() {
    let mut cci: Cci<GlobalPortId> = Cci::new_cmd_complete();
    cci.set_eom(true)
        .set_vendor_message(true)
        .set_security_req(true)
        .set_fw_update_req(true);

    let raw: u32 = cci.into();

    assert_eq!(raw & 0x0000_0001, 0, "UCSI 1.2 Table 3-2 reserves CCI bit 0");
    assert_eq!(raw & 0x01FF_0000, 0, "UCSI 1.2 Table 3-2 reserves CCI bits 24:16");
}

/// UCSI 1.2 Table 4-24 (GET_ALTERNATE_MODES Command), Number of Alternate Modes:
///
/// > to return is the value in this field plus 1. The maximum
/// > value of this field is 1.
///
/// The wire field is a count minus one, so the only legal encodings are 0 (one mode)
/// and 1 (two modes). `Args::num_modes` is documented as a number of modes, so asking
/// for one mode must encode 0 and asking for two must encode 1.
#[test]
fn get_alternate_modes_encodes_count_minus_one() {
    let one = get_alternate_modes::ArgBitsRaw::from(get_alternate_modes::Args {
        recipient: Recipient::Connector,
        connector_number: 1,
        mode_offset: 0,
        num_modes: 1,
    });
    assert_eq!(
        one.num_modes(),
        0,
        "requesting 1 alternate mode must encode 0 (UCSI 1.2 Table 4-24: \"the value in this field plus 1\")"
    );

    let two = get_alternate_modes::ArgBitsRaw::from(get_alternate_modes::Args {
        recipient: Recipient::Connector,
        connector_number: 1,
        mode_offset: 0,
        num_modes: 2,
    });
    assert_eq!(
        two.num_modes(),
        1,
        "requesting 2 alternate modes must encode 1 (UCSI 1.2 Table 4-24: \"The maximum value of this field is 1\")"
    );
}
