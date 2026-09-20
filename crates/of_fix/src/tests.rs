#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_parses_heartbeat() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::BEGIN_STRING),
            Some(b"FIX.4.4".as_slice())
        );
        assert_eq!(message.msg_type(), Some(b"0".as_slice()));
        assert_eq!(message.msg_seq_num(), Some(1));
        assert!(!message.poss_dup());
        assert!(message.debug_render().contains("35=0|"));
    }

    #[test]
    fn detects_body_length_mismatch() {
        let raw = b"8=FIX.4.4\x019=1\x0135=0\x0134=1\x0110=222\x01";
        let mut scratch = [FixFieldView::empty(); 16];
        let err = parse_message(raw, &mut scratch).expect_err("body length mismatch");
        assert!(matches!(err, FixParseError::BodyLengthMismatch { .. }));
    }

    #[test]
    fn detects_checksum_mismatch() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let len = raw.len();
        raw[len - 3] = b'9';
        let mut scratch = [FixFieldView::empty(); 16];
        let err = parse_message(&raw, &mut scratch).expect_err("checksum mismatch");
        assert!(matches!(err, FixParseError::ChecksumMismatch { .. }));
    }

    #[test]
    fn rejects_too_small_scratch() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let mut scratch = [FixFieldView::empty(); 2];
        let err = parse_message(&raw, &mut scratch).expect_err("scratch too small");
        assert!(matches!(err, FixParseError::ScratchTooSmall { .. }));
    }

    #[test]
    fn rejects_reserved_encode_tags() {
        let mut raw = Vec::new();
        let err = encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[(FixTag::CHECK_SUM, b"001".as_slice())],
        )
        .expect_err("reserved tag");
        assert_eq!(err, FixEncodeError::ReservedTag(FixTag::CHECK_SUM));
    }

    #[test]
    fn rejects_soh_in_values() {
        let mut raw = Vec::new();
        let err = encode_message(&mut raw, b"FIX.4.4", b"D\x01", &[]).expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::MSG_TYPE));
    }

    #[test]
    fn typed_encoder_decoder_round_trip() {
        let mut encoder = FixEncoder::with_capacity(128);
        let raw = encoder
            .encode_typed(
                FixVersion::Fix44,
                FixMsgType::NEW_ORDER_SINGLE,
                &[
                    (FixTag::MSG_SEQ_NUM, b"7".as_slice()),
                    (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
                ],
            )
            .expect("encode");

        let decoder = FixDecoder::new();
        let mut scratch = [FixFieldView::empty(); 16];
        let message = decoder.parse(raw, &mut scratch).expect("parse");

        assert_eq!(message.version(), Some(FixVersion::Fix44));
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
        assert_eq!(message.msg_seq_num(), Some(7));
    }

    #[test]
    fn repeating_group_round_trips_without_allocating_group_entries() {
        static PARTY_FIELDS: &[FixTag] = &[FixTag(448), FixTag(447), FixTag(452)];
        let definition = FixRepeatingGroupDefinition::new(FixTag(453), FixTag(448), PARTY_FIELDS);
        let first = [
            (FixTag(448), b"PARTY-A".as_slice()),
            (FixTag(447), b"D".as_slice()),
            (FixTag(452), b"1".as_slice()),
        ];
        let second = [
            (FixTag(448), b"PARTY-B".as_slice()),
            (FixTag(447), b"D".as_slice()),
            (FixTag(452), b"3".as_slice()),
        ];
        let groups = [&first[..], &second[..]];
        let mut raw = Vec::new();
        encode_message_with_repeating_group(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[(FixTag::CL_ORD_ID, b"ORD-1".as_slice())],
            definition,
            &groups,
        )
        .expect("encode repeating group");

        let mut fields = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut fields).expect("parse");
        let mut entries = [FixRepeatingGroupEntry::empty(); 2];
        let view = message
            .repeating_group(definition, &mut entries)
            .expect("parse repeating group");

        assert_eq!(view.len(), 2);
        assert_eq!(
            view.get(0).and_then(|entry| entry.get(FixTag(448))),
            Some(b"PARTY-A".as_slice())
        );
        assert_eq!(view.iter().count(), 2);
        assert_eq!(view.get(1).map(|entry| entry.fields().len()), Some(3));
    }

    #[test]
    fn repeating_group_reports_scratch_and_shape_errors() {
        static PARTY_FIELDS: &[FixTag] = &[FixTag(448), FixTag(447)];
        let definition = FixRepeatingGroupDefinition::new(FixTag(453), FixTag(448), PARTY_FIELDS);
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[
                (FixTag(453), b"1".as_slice()),
                (FixTag(447), b"D".as_slice()),
            ],
        )
        .expect("encode malformed group");
        let mut fields = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut fields).expect("parse");
        let mut no_entries: [FixRepeatingGroupEntry; 0] = [];
        assert!(matches!(
            message.repeating_group(definition, &mut no_entries),
            Err(FixGroupError::ScratchTooSmall {
                required: 1,
                capacity: 0
            })
        ));

        let mut entries = [FixRepeatingGroupEntry::empty(); 1];
        assert!(matches!(
            message.repeating_group(definition, &mut entries),
            Err(FixGroupError::MissingDelimiter {
                index: 0,
                tag: FixTag(448)
            })
        ));

        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[
                (FixTag(453), b"1".as_slice()),
                (FixTag(448), b"PARTY".as_slice()),
                (FixTag(9999), b"ordinary".as_slice()),
                (FixTag(447), b"D".as_slice()),
            ],
        )
        .expect("encode non-contiguous group");
        let mut fields = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut fields).expect("parse non-contiguous group");
        let mut entries = [FixRepeatingGroupEntry::empty(); 1];
        assert!(matches!(
            message.repeating_group(definition, &mut entries),
            Err(FixGroupError::UnexpectedField {
                index: 6,
                tag: FixTag(447)
            })
        ));
    }

    #[test]
    fn repeating_group_encoder_rejects_invalid_entries() {
        static PARTY_FIELDS: &[FixTag] = &[FixTag(448), FixTag(447)];
        let definition = FixRepeatingGroupDefinition::new(FixTag(453), FixTag(448), PARTY_FIELDS);
        let mut raw = Vec::new();
        let missing_delimiter = [(FixTag(447), b"D".as_slice())];
        let groups = [&missing_delimiter[..]];
        assert!(matches!(
            encode_message_with_repeating_group(
                &mut raw,
                b"FIX.4.4",
                b"D",
                &[],
                definition,
                &groups,
            ),
            Err(FixEncodeError::MissingRepeatingGroupDelimiter {
                group: 0,
                tag: FixTag(448)
            })
        ));

        let invalid = [
            (FixTag(448), b"PARTY".as_slice()),
            (FixTag(9999), b"unsupported".as_slice()),
        ];
        let groups = [&invalid[..]];
        assert!(matches!(
            encode_message_with_repeating_group(
                &mut raw,
                b"FIX.4.4",
                b"D",
                &[],
                definition,
                &groups,
            ),
            Err(FixEncodeError::InvalidRepeatingGroupField {
                group: 0,
                index: 1,
                tag: FixTag(9999)
            })
        ));

        let duplicate_count = [(FixTag(453), b"caller-count".as_slice())];
        let groups: [&[(FixTag, &[u8])]; 0] = [];
        assert!(matches!(
            encode_message_with_repeating_group(
                &mut raw,
                b"FIX.4.4",
                b"D",
                &duplicate_count,
                definition,
                &groups,
            ),
            Err(FixEncodeError::DuplicateRepeatingGroupCountTag(FixTag(453)))
        ));

        let repeated_delimiter = [
            (FixTag(448), b"PARTY-A".as_slice()),
            (FixTag(447), b"D".as_slice()),
            (FixTag(448), b"PARTY-B".as_slice()),
        ];
        let groups = [&repeated_delimiter[..]];
        assert!(matches!(
            encode_message_with_repeating_group(
                &mut raw,
                b"FIX.4.4",
                b"D",
                &[],
                definition,
                &groups,
            ),
            Err(FixEncodeError::RepeatedRepeatingGroupDelimiter {
                group: 0,
                index: 2,
                tag: FixTag(448)
            })
        ));
    }

    #[test]
    fn dictionary_validates_required_tags() {
        static REQUIRED: &[FixTag] = &[FixTag::CL_ORD_ID, FixTag::SYMBOL, FixTag::SIDE];
        static RULES: &[FixMessageRule<'static>] = &[FixMessageRule::new(
            FixMsgType::NEW_ORDER_SINGLE,
            REQUIRED,
            &[],
        )];
        let dictionary = FixDictionary::new(FixVersion::Fix44, RULES);

        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[
                (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
            ],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let err = dictionary
            .validate(&message)
            .expect_err("missing side should fail");
        assert_eq!(
            err,
            FixProfileError::MissingRequiredTag {
                msg_type: FixMsgType::NEW_ORDER_SINGLE,
                tag: FixTag::SIDE,
            }
        );
    }

    #[test]
    fn dictionary_rejects_disallowed_tags() {
        static DISALLOWED: &[FixTag] = &[FixTag::TEXT];
        static RULES: &[FixMessageRule<'static>] =
            &[FixMessageRule::new(FixMsgType::HEARTBEAT, &[], DISALLOWED)];
        let dictionary = FixDictionary::new(FixVersion::Fix44, RULES);

        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::TEXT, b"no text here".as_slice())],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let err = dictionary
            .validate(&message)
            .expect_err("disallowed text should fail");
        assert_eq!(
            err,
            FixProfileError::DisallowedTag {
                msg_type: FixMsgType::HEARTBEAT,
                tag: FixTag::TEXT,
            }
        );
    }

    #[test]
    fn dictionary_rejects_version_mismatch() {
        static RULES: &[FixMessageRule<'static>] =
            &[FixMessageRule::new(FixMsgType::HEARTBEAT, &[], &[])];
        let dictionary = FixDictionary::new(FixVersion::Fix42, RULES);

        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        assert_eq!(
            dictionary.validate(&message),
            Err(FixProfileError::VersionMismatch {
                expected: FixVersion::Fix42,
                actual: FixVersion::Fix44,
            })
        );
    }

    #[test]
    fn sequence_tracker_accepts_expected_inbound() {
        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_inbound(1, false),
            Ok(FixSequenceAction::Accept { seq_no: 1 })
        );
        assert_eq!(tracker.next_inbound(), 2);
    }

    #[test]
    fn sequence_tracker_detects_gap_without_advancing() {
        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_inbound(3, false),
            Ok(FixSequenceAction::Gap {
                expected: 1,
                received: 3,
                resend: FixResendRange {
                    begin_seq_no: 1,
                    end_seq_no: 2,
                },
            })
        );
        assert_eq!(tracker.next_inbound(), 1);
    }

    #[test]
    fn sequence_tracker_marks_poss_dup_low_sequence_duplicate() {
        let mut tracker = FixSequenceTracker::from_next(5, 9);
        assert_eq!(
            tracker.observe_inbound(3, true),
            Ok(FixSequenceAction::Duplicate {
                seq_no: 3,
                expected: 5,
            })
        );
        assert_eq!(tracker.next_inbound(), 5);
    }

    #[test]
    fn sequence_tracker_marks_unflagged_low_sequence_too_low() {
        let mut tracker = FixSequenceTracker::from_next(5, 9);
        assert_eq!(
            tracker.observe_inbound(3, false),
            Ok(FixSequenceAction::TooLow {
                expected: 5,
                received: 3,
            })
        );
    }

    #[test]
    fn sequence_tracker_observes_parsed_message_sequence() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_message(&message),
            Ok(FixSequenceAction::Accept { seq_no: 1 })
        );
    }

    #[test]
    fn sequence_tracker_assigns_outbound_monotonically() {
        let mut tracker = FixSequenceTracker::from_next(1, 10);
        assert_eq!(tracker.assign_outbound(), 10);
        assert_eq!(tracker.assign_outbound(), 11);
        assert_eq!(tracker.next_outbound(), 12);
    }

    #[test]
    fn sequence_reset_advances_but_does_not_decrease() {
        let mut tracker = FixSequenceTracker::from_next(10, 1);
        tracker.apply_sequence_reset(15).expect("advance");
        assert_eq!(tracker.next_inbound(), 15);
        assert_eq!(
            tracker.apply_sequence_reset(14),
            Err(FixSequenceError::SequenceResetWouldDecrease {
                current: 15,
                requested: 14,
            })
        );
    }

    #[test]
    fn session_id_rejects_soh() {
        let err = FixSessionId::new(FixVersion::Fix44, b"CLIENT\x01", b"BROKER").expect_err("soh");
        assert_eq!(
            err,
            FixEncodeError::ValueContainsSoh(FixTag::SENDER_COMP_ID)
        );
    }

    #[test]
    fn sequence_snapshot_round_trips_tracker_state() {
        let session_id =
            FixSessionId::with_qualifier(FixVersion::Fix44, b"CLIENT", b"BROKER", b"A")
                .expect("session");
        let tracker = FixSequenceTracker::from_next(12, 34);
        let snapshot = tracker.snapshot(session_id, b"20260717").expect("snapshot");

        assert_eq!(snapshot.session_id(), session_id);
        assert_eq!(snapshot.trading_day(), b"20260717");
        assert_eq!(snapshot.next_inbound(), 12);
        assert_eq!(snapshot.next_outbound(), 34);

        let restored = FixSequenceTracker::from_snapshot(&snapshot);
        assert_eq!(restored.next_inbound(), 12);
        assert_eq!(restored.next_outbound(), 34);
    }

    #[test]
    fn sequence_snapshot_clamps_zero_counters() {
        let session_id =
            FixSessionId::new(FixVersion::Fix44, b"CLIENT", b"BROKER").expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 0, 0, b"20260717").expect("snapshot");
        assert_eq!(snapshot.next_inbound(), 1);
        assert_eq!(snapshot.next_outbound(), 1);
    }

    #[test]
    fn file_sequence_snapshot_store_saves_and_loads_latest() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let session_id =
            FixSessionId::with_qualifier(FixVersion::Fix44, b"CLIENT", b"BROKER", b"PRIMARY")
                .expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 42, 77, b"20260726").expect("snapshot");
        let mut store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");

        let manifest = store.save_snapshot(&snapshot).expect("save");
        assert_eq!(manifest.next_inbound, 42);
        assert_eq!(manifest.next_outbound, 77);
        assert!(manifest.bytes > 0);

        let loaded = store.load_latest().expect("load").expect("snapshot");
        assert!(loaded.validate_checksum());
        assert_eq!(loaded.session_id().version(), FixVersion::Fix44);
        assert_eq!(loaded.session_id().sender_comp_id(), b"CLIENT");
        assert_eq!(loaded.session_id().target_comp_id(), b"BROKER");
        assert_eq!(loaded.session_id().qualifier(), b"PRIMARY");
        assert_eq!(loaded.trading_day(), b"20260726");

        let borrowed = loaded.as_borrowed().expect("borrowed");
        let restored = FixSequenceTracker::from_snapshot(&borrowed);
        assert_eq!(restored.next_inbound(), 42);
        assert_eq!(restored.next_outbound(), 77);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_sequence_snapshot_store_returns_none_when_empty() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-empty-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");

        assert!(store.load_latest().expect("load").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_sequence_snapshot_store_rejects_corrupt_checksum() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-corrupt-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let session_id =
            FixSessionId::new(FixVersion::Fix42, b"CLIENT", b"BROKER").expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 12, 21, b"20260726").expect("snapshot");
        let mut store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");
        store.save_snapshot(&snapshot).expect("save");

        let path = store.snapshot_path();
        let mut bytes = fs::read(&path).expect("read");
        let last = bytes.last_mut().expect("byte");
        *last ^= 0x01;
        fs::write(&path, bytes).expect("write");

        let err = store.load_latest().expect_err("checksum");
        assert!(matches!(
            err,
            FixSequenceStoreError::ChecksumMismatch { .. }
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sequence_tracker_resets_to_one() {
        let mut tracker = FixSequenceTracker::from_next(99, 100);
        tracker.reset_to_one();
        assert_eq!(tracker.next_inbound(), 1);
        assert_eq!(tracker.next_outbound(), 1);
    }

    #[test]
    fn resend_store_plans_replay_and_gap_fills() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(8, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Administrative, b"admin-2")
            .expect("seq 2");
        store
            .record_sent(3, FixSentMessageKind::Reject, b"reject-3")
            .expect("seq 3");
        store
            .record_sent(5, FixSentMessageKind::Application, b"app-5")
            .expect("seq 5");

        let mut actions = Vec::new();
        let summary = store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 5,
            },
            &mut actions,
        );

        assert_eq!(summary.replay_messages(), 3);
        assert_eq!(summary.gap_fill_messages(), 2);
        assert_eq!(summary.gap_fill_sequences(), 2);
        assert_eq!(
            actions,
            vec![
                FixResendAction::Replay {
                    seq_no: 1,
                    raw: b"app-1"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 2,
                    end_seq_no: 2
                },
                FixResendAction::Replay {
                    seq_no: 3,
                    raw: b"reject-3"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 4,
                    end_seq_no: 4
                },
                FixResendAction::Replay {
                    seq_no: 5,
                    raw: b"app-5"
                },
            ]
        );
    }

    #[test]
    fn resend_store_uses_newest_sequence_for_open_ended_range() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(8, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Administrative, b"admin-2")
            .expect("seq 2");

        let mut actions = vec![FixResendAction::GapFill {
            begin_seq_no: 99,
            end_seq_no: 99,
        }];
        let summary = store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 0,
            },
            &mut actions,
        );

        assert_eq!(summary.replay_messages(), 1);
        assert_eq!(summary.gap_fill_sequences(), 1);
        assert_eq!(
            actions,
            vec![
                FixResendAction::Replay {
                    seq_no: 1,
                    raw: b"app-1"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 2,
                    end_seq_no: 2
                },
            ]
        );
    }

    #[test]
    fn resend_store_eviction_turns_old_sequences_into_gap_fill() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(2, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Application, b"app-2")
            .expect("seq 2");
        let retention = store
            .record_sent(3, FixSentMessageKind::Application, b"app-3")
            .expect("seq 3");
        assert!(retention.retained());
        assert_eq!(retention.evicted_messages(), 1);

        let metrics = store.metrics();
        assert_eq!(metrics.retained_messages(), 2);
        assert_eq!(metrics.oldest_seq_no(), Some(2));
        assert_eq!(metrics.newest_seq_no(), Some(3));
        assert_eq!(metrics.evicted_messages(), 1);

        let mut actions = Vec::new();
        store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 3,
            },
            &mut actions,
        );
        assert_eq!(
            actions,
            vec![
                FixResendAction::GapFill {
                    begin_seq_no: 1,
                    end_seq_no: 1
                },
                FixResendAction::Replay {
                    seq_no: 2,
                    raw: b"app-2"
                },
                FixResendAction::Replay {
                    seq_no: 3,
                    raw: b"app-3"
                },
            ]
        );
    }

    #[test]
    fn resend_store_reports_disabled_or_oversized_drops() {
        let mut disabled = FixResendStore::new(FixResendStoreConfig::new(0, 1024));
        let retention = disabled
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("disabled retention");
        assert!(!retention.retained());
        assert_eq!(disabled.metrics().dropped_messages(), 1);

        let mut bounded = FixResendStore::new(FixResendStoreConfig::new(4, 4));
        let retention = bounded
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("oversized retention");
        assert!(!retention.retained());
        assert_eq!(bounded.metrics().dropped_bytes(), 5);
    }

    #[test]
    fn resend_store_rejects_non_increasing_sequences() {
        let mut store = FixResendStore::default();
        store
            .record_sent(10, FixSentMessageKind::Application, b"app-10")
            .expect("seq 10");
        let err = store
            .record_sent(10, FixSentMessageKind::Application, b"app-10-again")
            .expect_err("same sequence");
        assert_eq!(
            err,
            FixResendStoreError::SequenceRegression {
                latest: 10,
                received: 10
            }
        );
    }

    #[test]
    fn durable_resend_store_reopens_and_rebuilds_planner() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-fix-durable-resend-{}.log",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        {
            let mut durable = FileFixDurableResendStore::open(
                FixDurableResendStoreConfig::new(&path).with_sync_on_record(false),
            )
            .expect("durable");
            let first = durable
                .record_sent(1, FixSentMessageKind::Application, b"app-1")
                .expect("seq 1");
            assert_eq!(first.offset, 0);
            assert!(first.bytes > 0);
            durable
                .record_sent(2, FixSentMessageKind::Administrative, b"admin-2")
                .expect("seq 2");
            durable
                .record_sent(3, FixSentMessageKind::Reject, b"reject-3")
                .expect("seq 3");
        }

        let durable = FileFixDurableResendStore::open(
            FixDurableResendStoreConfig::new(&path).with_sync_on_record(false),
        )
        .expect("reopen");
        let mut store = FixResendStore::new(FixResendStoreConfig::new(8, 1024));
        let report = durable.load_into(&mut store).expect("load");
        assert_eq!(report.records, 3);
        assert_eq!(report.first_seq_no, Some(1));
        assert_eq!(report.last_seq_no, Some(3));
        assert_eq!(report.retained_messages, 3);

        let mut actions = Vec::new();
        let summary = store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 3,
            },
            &mut actions,
        );
        assert_eq!(summary.replay_messages(), 2);
        assert_eq!(summary.gap_fill_sequences(), 1);
        assert_eq!(
            actions,
            vec![
                FixResendAction::Replay {
                    seq_no: 1,
                    raw: b"app-1"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 2,
                    end_seq_no: 2
                },
                FixResendAction::Replay {
                    seq_no: 3,
                    raw: b"reject-3"
                },
            ]
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn durable_resend_store_rejects_corrupt_existing_log() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-fix-durable-resend-corrupt-{}.log",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let mut durable = FileFixDurableResendStore::open(
            FixDurableResendStoreConfig::new(&path).with_sync_on_record(false),
        )
        .expect("durable");
        durable
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        drop(durable);

        let mut bytes = fs::read(&path).expect("read");
        let last = bytes.last_mut().expect("byte");
        *last ^= 0x01;
        fs::write(&path, bytes).expect("write");

        let err = FileFixDurableResendStore::open(
            FixDurableResendStoreConfig::new(&path).with_sync_on_record(false),
        )
        .expect_err("corrupt");
        assert!(matches!(
            err,
            FixDurableResendStoreError::RawHashMismatch { .. }
                | FixDurableResendStoreError::FrameChecksumMismatch { .. }
        ));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn durable_resend_store_rejects_sequence_regression() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-fix-durable-resend-regression-{}.log",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let mut durable = FileFixDurableResendStore::open(
            FixDurableResendStoreConfig::new(&path).with_sync_on_record(false),
        )
        .expect("durable");
        durable
            .record_sent(7, FixSentMessageKind::Application, b"app-7")
            .expect("seq 7");

        let err = durable
            .record_sent(7, FixSentMessageKind::Application, b"app-7-again")
            .expect_err("same seq");
        assert_eq!(
            err,
            FixDurableResendStoreError::SequenceRegression {
                latest: 7,
                received: 7
            }
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn encodes_poss_dup_replay_with_orig_sending_time() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:05.000");
        let order = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:05.000",
            b"1.25",
            FixOrdType::Limit,
        )
        .with_price(b"65000.5");
        let mut original = Vec::new();
        encode_new_order_single(&mut original, FixVersion::Fix44, header, order)
            .expect("original order");
        let mut scratch = [FixFieldView::empty(); 32];
        let original_view = parse_message(&original, &mut scratch).expect("parse original");

        let mut replay = Vec::new();
        encode_poss_dup_replay(&mut replay, &original_view, b"20260717-12:00:06.000")
            .expect("replay");
        let mut replay_scratch = [FixFieldView::empty(); 40];
        let replay_view = parse_message(&replay, &mut replay_scratch).expect("parse replay");

        assert_eq!(replay_view.msg_seq_num(), Some(7));
        assert_eq!(
            replay_view.get(FixTag::POSS_DUP_FLAG),
            Some(b"Y".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::SENDING_TIME),
            Some(b"20260717-12:00:06.000".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::ORIG_SENDING_TIME),
            Some(b"20260717-12:00:05.000".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
    }

    #[test]
    fn poss_dup_replay_preserves_existing_orig_sending_time() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:05.000");
        let mut original = Vec::new();
        encode_heartbeat(&mut original, FixVersion::Fix44, header, None).expect("heartbeat");
        let mut scratch = [FixFieldView::empty(); 16];
        let original_view = parse_message(&original, &mut scratch).expect("parse original");

        let mut first_replay = Vec::new();
        encode_poss_dup_replay(&mut first_replay, &original_view, b"20260717-12:00:06.000")
            .expect("first replay");
        let mut first_scratch = [FixFieldView::empty(); 20];
        let first_view = parse_message(&first_replay, &mut first_scratch).expect("parse first");

        let mut second_replay = Vec::new();
        encode_poss_dup_replay(&mut second_replay, &first_view, b"20260717-12:00:07.000")
            .expect("second replay");
        let mut second_scratch = [FixFieldView::empty(); 20];
        let second_view = parse_message(&second_replay, &mut second_scratch).expect("parse second");

        assert_eq!(
            second_view.get(FixTag::SENDING_TIME),
            Some(b"20260717-12:00:07.000".as_slice())
        );
        assert_eq!(
            second_view.get(FixTag::ORIG_SENDING_TIME),
            Some(b"20260717-12:00:05.000".as_slice())
        );
    }

    #[test]
    fn poss_dup_replay_requires_source_sending_time() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[
                (FixTag::SENDER_COMP_ID, b"CLIENT".as_slice()),
                (FixTag::TARGET_COMP_ID, b"BROKER".as_slice()),
                (FixTag::MSG_SEQ_NUM, b"1".as_slice()),
            ],
        )
        .expect("source without sending time");
        let mut scratch = [FixFieldView::empty(); 16];
        let view = parse_message(&raw, &mut scratch).expect("parse source");
        let err = encode_poss_dup_replay(&mut Vec::new(), &view, b"20260717-12:00:06.000")
            .expect_err("missing sending time");
        assert_eq!(
            err,
            FixEncodeError::MissingRequiredTag(FixTag::SENDING_TIME)
        );
    }

    #[test]
    fn parses_session_reject_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"3",
            &[
                (FixTag::REF_SEQ_NUM, b"12".as_slice()),
                (FixTag::REF_TAG_ID, b"55".as_slice()),
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::SESSION_REJECT_REASON, b"1".as_slice()),
                (FixTag::TEXT, b"missing symbol".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let reject = parse_session_reject(&message).expect("reject");

        assert_eq!(reject.ref_seq_num(), 12);
        assert_eq!(reject.ref_tag_id(), Some(FixTag::SYMBOL));
        assert_eq!(reject.ref_msg_type(), Some(b"D".as_slice()));
        assert_eq!(reject.session_reject_reason(), Some(1));
        assert_eq!(reject.text(), Some(b"missing symbol".as_slice()));
    }

    #[test]
    fn session_reject_requires_ref_seq_num() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"3", &[]).expect("encode");

        let mut scratch = [FixFieldView::empty(); 8];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_session_reject(&message),
            Err(FixRejectParseError::MissingTag(FixTag::REF_SEQ_NUM))
        );
    }

    #[test]
    fn parses_business_message_reject_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"j",
            &[
                (FixTag::REF_SEQ_NUM, b"21".as_slice()),
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::BUSINESS_REJECT_REF_ID, b"ORD-1".as_slice()),
                (FixTag::BUSINESS_REJECT_REASON, b"3".as_slice()),
                (FixTag::TEXT, b"unsupported order".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let reject = parse_business_message_reject(&message).expect("reject");

        assert_eq!(reject.ref_seq_num(), Some(21));
        assert_eq!(reject.ref_msg_type(), b"D".as_slice());
        assert_eq!(reject.business_reject_ref_id(), Some(b"ORD-1".as_slice()));
        assert_eq!(reject.business_reject_reason(), 3);
        assert_eq!(reject.text(), Some(b"unsupported order".as_slice()));
    }

    #[test]
    fn business_message_reject_validates_required_numeric_reason() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"j",
            &[
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::BUSINESS_REJECT_REASON, b"bad".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_business_message_reject(&message),
            Err(FixRejectParseError::InvalidNumber(
                FixTag::BUSINESS_REJECT_REASON
            ))
        );
    }

    #[test]
    fn encodes_logon_with_required_admin_fields() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 1, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_logon(&mut raw, FixVersion::Fix44, header, 30, true).expect("logon");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::LOGON));
        assert_eq!(
            message.get(FixTag::SENDER_COMP_ID),
            Some(b"CLIENT".as_slice())
        );
        assert_eq!(
            message.get(FixTag::TARGET_COMP_ID),
            Some(b"BROKER".as_slice())
        );
        assert_eq!(message.get(FixTag::ENCRYPT_METHOD), Some(b"0".as_slice()));
        assert_eq!(message.get(FixTag::HEART_BT_INT), Some(b"30".as_slice()));
        assert_eq!(
            message.get(FixTag::RESET_SEQ_NUM_FLAG),
            Some(b"Y".as_slice())
        );
    }

    #[test]
    fn encodes_heartbeat_with_test_request_id() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 2, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_heartbeat(&mut raw, FixVersion::Fix44, header, Some(b"T1")).expect("heartbeat");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::HEARTBEAT));
        assert_eq!(message.get(FixTag::TEST_REQ_ID), Some(b"T1".as_slice()));
    }

    #[test]
    fn transcript_capture_records_parsed_messages() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 2, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_heartbeat(&mut raw, FixVersion::Fix44, header, Some(b"T1")).expect("heartbeat");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(4, 1024, true));
        let retention = capture
            .record_message(
                FixTranscriptDirection::Outbound,
                1_784_275_200_000_000_000,
                &message,
            )
            .expect("record");

        assert!(retention.retained());
        assert!(retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 1);
        assert_eq!(metrics.retained_records(), 1);
        assert_eq!(metrics.retained_raw_bytes(), raw.len() as u64);
        assert_ne!(metrics.rolling_hash(), FNV_OFFSET_BASIS);
        let record = capture.records().next().expect("record");
        assert_eq!(record.ordinal(), 1);
        assert_eq!(record.direction(), FixTranscriptDirection::Outbound);
        assert_eq!(record.seq_no(), Some(2));
        assert_eq!(record.msg_type(), FixMsgType::HEARTBEAT.as_bytes());
        assert_eq!(record.raw_checksum(), checksum(&raw));
        assert_eq!(record.raw(), raw.as_slice());
    }

    #[test]
    fn transcript_capture_evicts_to_bounds() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(2, 64, true));
        let first = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                1,
                Some(1),
                b"0",
                b"8=FIX.4.4\x0135=0\x01",
            )
            .expect("record first");
        assert!(first.retained());
        let second = capture
            .record_frame(
                FixTranscriptDirection::Outbound,
                2,
                Some(2),
                b"1",
                b"8=FIX.4.4\x0135=1\x01",
            )
            .expect("record second");
        assert!(second.retained());
        let third = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                3,
                Some(3),
                b"2",
                b"8=FIX.4.4\x0135=2\x01",
            )
            .expect("record third");

        assert_eq!(third.evicted_records(), 1);
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 3);
        assert_eq!(metrics.retained_records(), 2);
        assert_eq!(metrics.evicted_records(), 1);
        assert_eq!(metrics.oldest_ordinal(), Some(2));
        assert_eq!(metrics.newest_ordinal(), Some(3));
    }

    #[test]
    fn transcript_capture_can_keep_metadata_without_raw() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(4, 8, true));
        let retention = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                1,
                Some(1),
                b"D",
                b"this raw frame is intentionally too large",
            )
            .expect("record");

        assert!(retention.retained());
        assert!(!retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.retained_records(), 1);
        assert_eq!(metrics.retained_raw_bytes(), 0);
        assert_eq!(
            metrics.dropped_raw_bytes(),
            "this raw frame is intentionally too large".len() as u64
        );
        let record = capture.records().next().expect("record");
        assert_eq!(
            record.raw_len(),
            "this raw frame is intentionally too large".len()
        );
        assert!(!record.raw_retained());
        assert!(record.raw().is_empty());
    }

    #[test]
    fn transcript_capture_respects_disabled_record_retention() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(0, 1024, true));
        let retention = capture
            .record_frame(FixTranscriptDirection::Inbound, 1, None, b"0", b"raw")
            .expect("record");

        assert!(!retention.retained());
        assert!(!retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 1);
        assert_eq!(metrics.retained_records(), 0);
        assert_eq!(metrics.dropped_records(), 1);
        assert_eq!(metrics.dropped_raw_bytes(), 3);
        assert_ne!(metrics.rolling_hash(), FNV_OFFSET_BASIS);
    }

    #[test]
    fn transcript_message_type_is_bounded() {
        let mut capture = FixTranscriptCapture::default();
        assert_eq!(
            capture.record_frame(
                FixTranscriptDirection::Inbound,
                1,
                None,
                b"TOO-LONG-MSG-TYPE",
                b"raw",
            ),
            Err(FixTranscriptError::MsgTypeTooLong {
                capacity: 8,
                actual: 17,
            })
        );
    }

    #[test]
    fn encodes_resend_request_range() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 3, b"20260717-12:00:02.000");
        let mut raw = Vec::new();
        encode_resend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            FixResendRange {
                begin_seq_no: 4,
                end_seq_no: 9,
            },
        )
        .expect("resend request");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::RESEND_REQUEST));
        assert_eq!(message.begin_seq_no(), Some(4));
        assert_eq!(message.end_seq_no(), Some(9));
    }

    #[test]
    fn encodes_sequence_reset_gap_fill() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 4, b"20260717-12:00:03.000");
        let mut raw = Vec::new();
        encode_sequence_reset_gap_fill(&mut raw, FixVersion::Fix44, header, 12).expect("gap fill");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::SEQUENCE_RESET));
        assert!(message.gap_fill());
        assert_eq!(message.new_seq_no(), Some(12));
    }

    #[test]
    fn logout_builder_rejects_soh_in_text() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 5, b"20260717-12:00:04.000");
        let mut raw = Vec::new();
        let err = encode_logout(
            &mut raw,
            FixVersion::Fix44,
            header,
            Some(b"bad\x01text".as_slice()),
        )
        .expect_err("soh should fail");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::TEXT));
    }

    #[test]
    fn encodes_new_order_single() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 6, b"20260717-12:00:05.000");
        let request = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:05.000",
            b"1.25",
            FixOrdType::Limit,
        )
        .with_account(b"ACC")
        .with_price(b"65000.5")
        .with_stop_px(b"64950")
        .with_time_in_force(FixTimeInForce::Day);

        let mut raw = Vec::new();
        encode_new_order_single(&mut raw, FixVersion::Fix44, header, request).expect("new order");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1.25".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64950".as_slice()));
    }

    #[test]
    fn encodes_order_cancel_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:06.000");
        let request = FixOrderCancelRequest::new(
            b"ORD-1",
            b"ORD-1-CXL",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:06.000",
        )
        .with_account(b"ACC");

        let mut raw = Vec::new();
        encode_order_cancel_request(&mut raw, FixVersion::Fix44, header, request).expect("cancel");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.typed_msg_type(),
            Some(FixMsgType::ORDER_CANCEL_REQUEST)
        );
        assert_eq!(
            message.get(FixTag::ORIG_CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::CL_ORD_ID),
            Some(b"ORD-1-CXL".as_slice())
        );
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
    }

    #[test]
    fn encodes_order_cancel_replace_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 8, b"20260717-12:00:07.000");
        let request = FixOrderCancelReplaceRequest::new(
            b"ORD-1",
            b"ORD-2",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:07.000",
            b"2.00",
            FixOrdType::Limit,
        )
        .with_account(b"ACC")
        .with_price(b"65100")
        .with_stop_px(b"65000")
        .with_time_in_force(FixTimeInForce::ImmediateOrCancel);

        let mut raw = Vec::new();
        encode_order_cancel_replace_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("replace");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.typed_msg_type(),
            Some(FixMsgType::ORDER_CANCEL_REPLACE_REQUEST)
        );
        assert_eq!(
            message.get(FixTag::ORIG_CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-2".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"2.00".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"65000".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"3".as_slice()));
    }

    #[test]
    fn encodes_order_status_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD-1").with_order_id(b"VENUE-1");
        let mut raw = Vec::new();
        encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("status request");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_STATUS_REQUEST.as_bytes())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_ID), Some(b"VENUE-1".as_slice()));
    }

    #[test]
    fn order_status_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD-1");
        let mut raw = Vec::new();
        encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("status request");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_ID), None);
    }

    #[test]
    fn order_status_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD\x01");
        let mut raw = Vec::new();
        let err = encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::CL_ORD_ID));
    }

    #[test]
    fn encodes_order_mass_cancel_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::Security,
            b"20260717-12:00:09.000",
        )
        .with_secondary_cl_ord_id(b"ALT-1")
        .with_trading_session_id(b"REG")
        .with_trading_session_sub_id(b"AM")
        .with_symbol(b"BTCUSDT")
        .with_side(FixOrderSide::Buy)
        .with_text(b"cancel symbol");
        let mut raw = Vec::new();
        encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass cancel");
        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_MASS_CANCEL_REQUEST.as_bytes())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"MASS-1".as_slice()));
        assert_eq!(
            message.get(FixTag::MASS_CANCEL_REQUEST_TYPE),
            Some(b"1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::TRANSACT_TIME),
            Some(b"20260717-12:00:09.000".as_slice())
        );
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::TEXT), Some(b"cancel symbol".as_slice()));
    }

    #[test]
    fn order_mass_cancel_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::AllOrders,
            b"20260717-12:00:09.000",
        );
        let mut raw = Vec::new();
        encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass cancel");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::MASS_CANCEL_REQUEST_TYPE),
            Some(b"7".as_slice())
        );
        assert_eq!(message.get(FixTag::SYMBOL), None);
    }

    #[test]
    fn order_mass_cancel_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::Security,
            b"20260717-12:00:09.000",
        )
        .with_text(b"bad\x01text");
        let mut raw = Vec::new();
        let err = encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::TEXT));
    }

    #[test]
    fn encodes_order_mass_status_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS-1", FixMassStatusReqType::Security)
            .with_account(b"ACC")
            .with_acct_id_source(b"1")
            .with_trading_session_id(b"REG")
            .with_trading_session_sub_id(b"AM")
            .with_symbol(b"BTCUSDT")
            .with_side(FixOrderSide::Sell);
        let mut raw = Vec::new();
        encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass status");
        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_MASS_STATUS_REQUEST.as_bytes())
        );
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_ID),
            Some(b"MS-1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_TYPE),
            Some(b"1".as_slice())
        );
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
    }

    #[test]
    fn order_mass_status_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS-1", FixMassStatusReqType::AllOrders);
        let mut raw = Vec::new();
        encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass status");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_TYPE),
            Some(b"7".as_slice())
        );
        assert_eq!(message.get(FixTag::ACCOUNT), None);
    }

    #[test]
    fn order_mass_status_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS\x01", FixMassStatusReqType::AllOrders);
        let mut raw = Vec::new();
        let err = encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(
            err,
            FixEncodeError::ValueContainsSoh(FixTag::MASS_STATUS_REQ_ID)
        );
    }

    #[test]
    fn order_builder_rejects_soh_in_symbol() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:08.000");
        let request = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTC\x01USDT",
            FixOrderSide::Buy,
            b"20260717-12:00:08.000",
            b"1",
            FixOrdType::Market,
        );
        let mut raw = Vec::new();
        let err = encode_new_order_single(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh should fail");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::SYMBOL));
    }
}
