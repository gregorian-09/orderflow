# `of_fix` Reference

> Generated from `crates/of_fix/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.1.0`<br>
**Description:** Low-allocation FIX tag-value codec primitives for Orderflow execution adapters<br>
**Source:** [`crates/of_fix/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_fix/src)<br>
**Generated Rustdoc:** [open `of_fix` Rustdoc](https://docs.rs/of_fix/0.1.0/of_fix/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- No crate-defined features.

## Local Dependencies

- No local workspace dependencies.

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `const` | `SOH` | FIX field delimiter byte | [`crates/of_fix/src/lib.rs:25`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L25) | `present` |
| `struct` | `FixTag` | Numeric FIX tag identifier | [`crates/of_fix/src/lib.rs:29`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L29) | `present` |
| `const` | `BEGIN_STRING` | `BeginString(8)` | [`crates/of_fix/src/lib.rs:33`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L33) | `present` |
| `const` | `ACCOUNT` | `Account(1)` | [`crates/of_fix/src/lib.rs:35`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L35) | `present` |
| `const` | `BODY_LENGTH` | `BodyLength(9)` | [`crates/of_fix/src/lib.rs:37`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L37) | `present` |
| `const` | `BEGIN_SEQ_NO` | `BeginSeqNo(7)` | [`crates/of_fix/src/lib.rs:39`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L39) | `present` |
| `const` | `END_SEQ_NO` | `EndSeqNo(16)` | [`crates/of_fix/src/lib.rs:41`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L41) | `present` |
| `const` | `MSG_TYPE` | `MsgType(35)` | [`crates/of_fix/src/lib.rs:43`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L43) | `present` |
| `const` | `MSG_SEQ_NUM` | `MsgSeqNum(34)` | [`crates/of_fix/src/lib.rs:45`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L45) | `present` |
| `const` | `NEW_SEQ_NO` | `NewSeqNo(36)` | [`crates/of_fix/src/lib.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L47) | `present` |
| `const` | `POSS_DUP_FLAG` | `PossDupFlag(43)` | [`crates/of_fix/src/lib.rs:49`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L49) | `present` |
| `const` | `REF_SEQ_NUM` | `RefSeqNum(45)` | [`crates/of_fix/src/lib.rs:51`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L51) | `present` |
| `const` | `SENDER_COMP_ID` | `SenderCompID(49)` | [`crates/of_fix/src/lib.rs:53`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L53) | `present` |
| `const` | `SENDING_TIME` | `SendingTime(52)` | [`crates/of_fix/src/lib.rs:55`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L55) | `present` |
| `const` | `TARGET_COMP_ID` | `TargetCompID(56)` | [`crates/of_fix/src/lib.rs:57`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L57) | `present` |
| `const` | `CL_ORD_ID` | `ClOrdID(11)` | [`crates/of_fix/src/lib.rs:59`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L59) | `present` |
| `const` | `ORIG_CL_ORD_ID` | `OrigClOrdID(41)` | [`crates/of_fix/src/lib.rs:61`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L61) | `present` |
| `const` | `ORDER_ID` | `OrderID(37)` | [`crates/of_fix/src/lib.rs:63`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L63) | `present` |
| `const` | `EXEC_ID` | `ExecID(17)` | [`crates/of_fix/src/lib.rs:65`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L65) | `present` |
| `const` | `EXEC_TYPE` | `ExecType(150)` | [`crates/of_fix/src/lib.rs:67`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L67) | `present` |
| `const` | `ORD_STATUS` | `OrdStatus(39)` | [`crates/of_fix/src/lib.rs:69`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L69) | `present` |
| `const` | `SYMBOL` | `Symbol(55)` | [`crates/of_fix/src/lib.rs:71`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L71) | `present` |
| `const` | `SIDE` | `Side(54)` | [`crates/of_fix/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L73) | `present` |
| `const` | `TRADING_SESSION_ID` | `TradingSessionID(336)` | [`crates/of_fix/src/lib.rs:75`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L75) | `present` |
| `const` | `ENCODED_TEXT_LEN` | `EncodedTextLen(354)` | [`crates/of_fix/src/lib.rs:77`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L77) | `present` |
| `const` | `ORDER_QTY` | `OrderQty(38)` | [`crates/of_fix/src/lib.rs:79`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L79) | `present` |
| `const` | `ORD_TYPE` | `OrdType(40)` | [`crates/of_fix/src/lib.rs:81`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L81) | `present` |
| `const` | `PRICE` | `Price(44)` | [`crates/of_fix/src/lib.rs:83`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L83) | `present` |
| `const` | `TIME_IN_FORCE` | `TimeInForce(59)` | [`crates/of_fix/src/lib.rs:85`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L85) | `present` |
| `const` | `STOP_PX` | `StopPx(99)` | [`crates/of_fix/src/lib.rs:87`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L87) | `present` |
| `const` | `LAST_QTY` | `LastQty(32)` | [`crates/of_fix/src/lib.rs:89`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L89) | `present` |
| `const` | `LAST_PX` | `LastPx(31)` | [`crates/of_fix/src/lib.rs:91`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L91) | `present` |
| `const` | `CUM_QTY` | `CumQty(14)` | [`crates/of_fix/src/lib.rs:93`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L93) | `present` |
| `const` | `LEAVES_QTY` | `LeavesQty(151)` | [`crates/of_fix/src/lib.rs:95`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L95) | `present` |
| `const` | `AVG_PX` | `AvgPx(6)` | [`crates/of_fix/src/lib.rs:97`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L97) | `present` |
| `const` | `TRANSACT_TIME` | `TransactTime(60)` | [`crates/of_fix/src/lib.rs:99`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L99) | `present` |
| `const` | `TEXT` | `Text(58)` | [`crates/of_fix/src/lib.rs:101`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L101) | `present` |
| `const` | `ENCRYPT_METHOD` | `EncryptMethod(98)` | [`crates/of_fix/src/lib.rs:103`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L103) | `present` |
| `const` | `TEST_REQ_ID` | `TestReqID(112)` | [`crates/of_fix/src/lib.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L105) | `present` |
| `const` | `ORIG_SENDING_TIME` | `OrigSendingTime(122)` | [`crates/of_fix/src/lib.rs:107`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L107) | `present` |
| `const` | `HEART_BT_INT` | `HeartBtInt(108)` | [`crates/of_fix/src/lib.rs:109`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L109) | `present` |
| `const` | `GAP_FILL_FLAG` | `GapFillFlag(123)` | [`crates/of_fix/src/lib.rs:111`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L111) | `present` |
| `const` | `RESET_SEQ_NUM_FLAG` | `ResetSeqNumFlag(141)` | [`crates/of_fix/src/lib.rs:113`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L113) | `present` |
| `const` | `REF_TAG_ID` | `RefTagID(371)` | [`crates/of_fix/src/lib.rs:115`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L115) | `present` |
| `const` | `REF_MSG_TYPE` | `RefMsgType(372)` | [`crates/of_fix/src/lib.rs:117`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L117) | `present` |
| `const` | `SESSION_REJECT_REASON` | `SessionRejectReason(373)` | [`crates/of_fix/src/lib.rs:119`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L119) | `present` |
| `const` | `BUSINESS_REJECT_REF_ID` | `BusinessRejectRefID(379)` | [`crates/of_fix/src/lib.rs:121`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L121) | `present` |
| `const` | `BUSINESS_REJECT_REASON` | `BusinessRejectReason(380)` | [`crates/of_fix/src/lib.rs:123`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L123) | `present` |
| `const` | `SECONDARY_CL_ORD_ID` | `SecondaryClOrdID(526)` | [`crates/of_fix/src/lib.rs:125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L125) | `present` |
| `const` | `MASS_CANCEL_REQUEST_TYPE` | `MassCancelRequestType(530)` | [`crates/of_fix/src/lib.rs:127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L127) | `present` |
| `const` | `MASS_STATUS_REQ_ID` | `MassStatusReqID(584)` | [`crates/of_fix/src/lib.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L129) | `present` |
| `const` | `MASS_STATUS_REQ_TYPE` | `MassStatusReqType(585)` | [`crates/of_fix/src/lib.rs:131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L131) | `present` |
| `const` | `TRADING_SESSION_SUB_ID` | `TradingSessionSubID(625)` | [`crates/of_fix/src/lib.rs:133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L133) | `present` |
| `const` | `ACCT_ID_SOURCE` | `AcctIDSource(660)` | [`crates/of_fix/src/lib.rs:135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L135) | `present` |
| `const` | `CHECK_SUM` | `CheckSum(10)` | [`crates/of_fix/src/lib.rs:137`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L137) | `present` |
| `enum` | `FixVersion` | Known FIX begin-string versions | [`crates/of_fix/src/lib.rs:149`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L149) | `present` |
| `fn` | `as_bytes` | Returns the wire begin-string bytes | [`crates/of_fix/src/lib.rs:166`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L166) | `present` |
| `fn` | `from_bytes` | Parses a known begin-string version | [`crates/of_fix/src/lib.rs:178`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L178) | `present` |
| `struct` | `FixMsgType` | FIX `MsgType(35)` identifier | [`crates/of_fix/src/lib.rs:203`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L203) | `present` |
| `const` | `HEARTBEAT` | `Heartbeat(0)` | [`crates/of_fix/src/lib.rs:207`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L207) | `present` |
| `const` | `TEST_REQUEST` | `TestRequest(1)` | [`crates/of_fix/src/lib.rs:209`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L209) | `present` |
| `const` | `RESEND_REQUEST` | `ResendRequest(2)` | [`crates/of_fix/src/lib.rs:211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L211) | `present` |
| `const` | `REJECT` | `Reject(3)` | [`crates/of_fix/src/lib.rs:213`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L213) | `present` |
| `const` | `SEQUENCE_RESET` | `SequenceReset(4)` | [`crates/of_fix/src/lib.rs:215`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L215) | `present` |
| `const` | `LOGOUT` | `Logout(5)` | [`crates/of_fix/src/lib.rs:217`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L217) | `present` |
| `const` | `EXECUTION_REPORT` | `ExecutionReport(8)` | [`crates/of_fix/src/lib.rs:219`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L219) | `present` |
| `const` | `ORDER_CANCEL_REJECT` | `OrderCancelReject(9)` | [`crates/of_fix/src/lib.rs:221`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L221) | `present` |
| `const` | `LOGON` | `Logon(A)` | [`crates/of_fix/src/lib.rs:223`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L223) | `present` |
| `const` | `NEW_ORDER_SINGLE` | `NewOrderSingle(D)` | [`crates/of_fix/src/lib.rs:225`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L225) | `present` |
| `const` | `ORDER_CANCEL_REQUEST` | `OrderCancelRequest(F)` | [`crates/of_fix/src/lib.rs:227`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L227) | `present` |
| `const` | `ORDER_CANCEL_REPLACE_REQUEST` | `OrderCancelReplaceRequest(G)` | [`crates/of_fix/src/lib.rs:229`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L229) | `present` |
| `const` | `ORDER_STATUS_REQUEST` | `OrderStatusRequest(H)` | [`crates/of_fix/src/lib.rs:231`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L231) | `present` |
| `const` | `BUSINESS_MESSAGE_REJECT` | `BusinessMessageReject(j)` | [`crates/of_fix/src/lib.rs:233`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L233) | `present` |
| `const` | `ORDER_MASS_CANCEL_REQUEST` | `OrderMassCancelRequest(q)` | [`crates/of_fix/src/lib.rs:235`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L235) | `present` |
| `const` | `ORDER_MASS_STATUS_REQUEST` | `OrderMassStatusRequest(AF)` | [`crates/of_fix/src/lib.rs:237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L237) | `present` |
| `fn` | `from_static` | Creates a message type from a static byte slice | [`crates/of_fix/src/lib.rs:243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L243) | `present` |
| `fn` | `from_bytes` | Parses a known message type | [`crates/of_fix/src/lib.rs:248`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L248) | `present` |
| `fn` | `as_bytes` | Returns the wire message-type bytes | [`crates/of_fix/src/lib.rs:271`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L271) | `present` |
| `fn` | `name` | Returns a human-readable message type name for diagnostics | [`crates/of_fix/src/lib.rs:276`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L276) | `present` |
| `enum` | `FixOrderSide` | Common FIX `Side(54)` values for order-entry builders | [`crates/of_fix/src/lib.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L308) | `present` |
| `fn` | `as_bytes` | Returns the wire value | [`crates/of_fix/src/lib.rs:319`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L319) | `present` |
| `fn` | `from_bytes` | Parses a common side value | [`crates/of_fix/src/lib.rs:328`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L328) | `present` |
| `enum` | `FixOrdType` | Common FIX `OrdType(40)` values for order-entry builders | [`crates/of_fix/src/lib.rs:341`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L341) | `present` |
| `fn` | `as_bytes` | Returns the wire value | [`crates/of_fix/src/lib.rs:354`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L354) | `present` |
| `fn` | `from_bytes` | Parses a common order type | [`crates/of_fix/src/lib.rs:364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L364) | `present` |
| `enum` | `FixTimeInForce` | Common FIX `TimeInForce(59)` values for order-entry builders | [`crates/of_fix/src/lib.rs:378`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L378) | `present` |
| `fn` | `as_bytes` | Returns the wire value | [`crates/of_fix/src/lib.rs:393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L393) | `present` |
| `fn` | `from_bytes` | Parses a common time-in-force value | [`crates/of_fix/src/lib.rs:404`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L404) | `present` |
| `enum` | `FixMassCancelRequestType` | Common FIX `MassCancelRequestType(530)` values | [`crates/of_fix/src/lib.rs:419`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L419) | `present` |
| `fn` | `as_bytes` | Returns the wire value | [`crates/of_fix/src/lib.rs:438`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L438) | `present` |
| `enum` | `FixMassStatusReqType` | Common FIX `MassStatusReqType(585)` values | [`crates/of_fix/src/lib.rs:454`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L454) | `present` |
| `fn` | `as_bytes` | Returns the wire value | [`crates/of_fix/src/lib.rs:475`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L475) | `present` |
| `struct` | `FixFieldView` | Borrowed FIX tag-value field | [`crates/of_fix/src/lib.rs:491`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L491) | `present` |
| `fn` | `empty` | Creates an empty field placeholder for scratch buffers | [`crates/of_fix/src/lib.rs:500`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L500) | `present` |
| `struct` | `FixRepeatingGroupDefinition` | Describes one flat FIX repeating group | [`crates/of_fix/src/lib.rs:516`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L516) | `present` |
| `fn` | `new` | Creates a flat repeating-group definition | [`crates/of_fix/src/lib.rs:524`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L524) | `present` |
| `fn` | `count_tag` | Returns the group count tag | [`crates/of_fix/src/lib.rs:533`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L533) | `present` |
| `fn` | `delimiter_tag` | Returns the first-field delimiter tag | [`crates/of_fix/src/lib.rs:538`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L538) | `present` |
| `fn` | `field_tags` | Returns all tags allowed inside each group entry | [`crates/of_fix/src/lib.rs:543`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L543) | `present` |
| `struct` | `FixRepeatingGroupEntry` | Caller-owned boundary storage for one parsed repeating-group entry | [`crates/of_fix/src/lib.rs:554`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L554) | `present` |
| `fn` | `empty` | Creates an empty boundary slot for caller-provided scratch storage | [`crates/of_fix/src/lib.rs:561`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L561) | `present` |
| `fn` | `field_range` | Returns the field range occupied by this entry in the parsed message | [`crates/of_fix/src/lib.rs:566`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L566) | `present` |
| `struct` | `FixRepeatingGroup` | Borrowed fields for one repeating-group entry | [`crates/of_fix/src/lib.rs:573`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L573) | `present` |
| `fn` | `fields` | Returns the entry fields in wire order | [`crates/of_fix/src/lib.rs:579`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L579) | `present` |
| `fn` | `get` | Returns the first value for `tag` in this entry | [`crates/of_fix/src/lib.rs:584`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L584) | `present` |
| `struct` | `FixRepeatingGroupView` | Borrowed view over the entries of one flat repeating group | [`crates/of_fix/src/lib.rs:594`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L594) | `present` |
| `fn` | `len` | Returns the number of parsed group entries | [`crates/of_fix/src/lib.rs:601`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L601) | `present` |
| `fn` | `is_empty` | Returns true when no entries were declared | [`crates/of_fix/src/lib.rs:606`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L606) | `present` |
| `fn` | `get` | Returns one entry by zero-based index | [`crates/of_fix/src/lib.rs:611`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L611) | `present` |
| `fn` | `iter` | Iterates over entries in wire order without allocating | [`crates/of_fix/src/lib.rs:619`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L619) | `present` |
| `struct` | `FixMessageView` | Borrowed view over a validated FIX message | [`crates/of_fix/src/lib.rs:628`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L628) | `present` |
| `fn` | `raw` | Returns the raw FIX frame bytes | [`crates/of_fix/src/lib.rs:635`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L635) | `present` |
| `fn` | `fields` | Returns parsed fields in wire order | [`crates/of_fix/src/lib.rs:640`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L640) | `present` |
| `fn` | `get` | Returns the first field value for `tag` | [`crates/of_fix/src/lib.rs:645`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L645) | `present` |
| `fn` | `msg_type` | Returns `MsgType(35)` | [`crates/of_fix/src/lib.rs:653`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L653) | `present` |
| `fn` | `typed_msg_type` | Returns `MsgType(35)` as a known typed message kind when recognized | [`crates/of_fix/src/lib.rs:658`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L658) | `present` |
| `fn` | `begin_string` | Returns `BeginString(8)` | [`crates/of_fix/src/lib.rs:663`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L663) | `present` |
| `fn` | `version` | Returns `BeginString(8)` as a known FIX version when recognized | [`crates/of_fix/src/lib.rs:668`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L668) | `present` |
| `fn` | `msg_seq_num` | Returns `MsgSeqNum(34)` parsed as `u64` | [`crates/of_fix/src/lib.rs:673`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L673) | `present` |
| `fn` | `poss_dup` | Returns true when `PossDupFlag(43)` is `Y` | [`crates/of_fix/src/lib.rs:678`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L678) | `present` |
| `fn` | `gap_fill` | Returns true when `GapFillFlag(123)` is `Y` | [`crates/of_fix/src/lib.rs:683`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L683) | `present` |
| `fn` | `new_seq_no` | Returns `NewSeqNo(36)` parsed as `u64` | [`crates/of_fix/src/lib.rs:688`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L688) | `present` |
| `fn` | `begin_seq_no` | Returns `BeginSeqNo(7)` parsed as `u64` | [`crates/of_fix/src/lib.rs:693`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L693) | `present` |
| `fn` | `end_seq_no` | Returns `EndSeqNo(16)` parsed as `u64` | [`crates/of_fix/src/lib.rs:698`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L698) | `present` |
| `fn` | `debug_render` | Renders a debug string with `\|` separators instead of SOH | [`crates/of_fix/src/lib.rs:706`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L706) | `present` |
| `fn` | `repeating_group` | Parses one flat repeating group using caller-provided boundary scratch | [`crates/of_fix/src/lib.rs:722`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L722) | `present` |
| `enum` | `FixGroupError` | Errors returned while interpreting a flat FIX repeating group | [`crates/of_fix/src/lib.rs:784`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L784) | `present` |
| `enum` | `FixParseError` | FIX parse and validation errors | [`crates/of_fix/src/lib.rs:844`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L844) | `present` |
| `enum` | `FixEncodeError` | FIX encode errors | [`crates/of_fix/src/lib.rs:910`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L910) | `present` |
| `enum` | `FixProfileError` | FIX dictionary/profile validation errors | [`crates/of_fix/src/lib.rs:979`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L979) | `present` |
| `enum` | `FixRejectParseError` | FIX reject-message parse errors | [`crates/of_fix/src/lib.rs:1040`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1040) | `present` |
| `struct` | `FixSessionRejectView` | Borrowed Session Reject `<3>` view | [`crates/of_fix/src/lib.rs:1063`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1063) | `present` |
| `fn` | `ref_seq_num` | Returns `RefSeqNum(45)` | [`crates/of_fix/src/lib.rs:1073`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1073) | `present` |
| `fn` | `ref_tag_id` | Returns `RefTagID(371)` when present | [`crates/of_fix/src/lib.rs:1078`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1078) | `present` |
| `fn` | `ref_msg_type` | Returns `RefMsgType(372)` when present | [`crates/of_fix/src/lib.rs:1083`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1083) | `present` |
| `fn` | `session_reject_reason` | Returns `SessionRejectReason(373)` when present | [`crates/of_fix/src/lib.rs:1088`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1088) | `present` |
| `fn` | `text` | Returns `Text(58)` when present | [`crates/of_fix/src/lib.rs:1093`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1093) | `present` |
| `struct` | `FixBusinessMessageRejectView` | Borrowed BusinessMessageReject `<j>` view | [`crates/of_fix/src/lib.rs:1100`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1100) | `present` |
| `fn` | `ref_seq_num` | Returns `RefSeqNum(45)` when present | [`crates/of_fix/src/lib.rs:1110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1110) | `present` |
| `fn` | `ref_msg_type` | Returns required `RefMsgType(372)` | [`crates/of_fix/src/lib.rs:1115`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1115) | `present` |
| `fn` | `business_reject_ref_id` | Returns `BusinessRejectRefID(379)` when present | [`crates/of_fix/src/lib.rs:1120`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1120) | `present` |
| `fn` | `business_reject_reason` | Returns required `BusinessRejectReason(380)` | [`crates/of_fix/src/lib.rs:1125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1125) | `present` |
| `fn` | `text` | Returns `Text(58)` when present | [`crates/of_fix/src/lib.rs:1130`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1130) | `present` |
| `enum` | `FixSessionState` | FIX session lifecycle state | [`crates/of_fix/src/lib.rs:1138`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1138) | `present` |
| `struct` | `FixResendRange` | Resend range requested after an inbound sequence gap | [`crates/of_fix/src/lib.rs:1162`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1162) | `present` |
| `enum` | `FixSequenceAction` | Result of observing an inbound sequence number | [`crates/of_fix/src/lib.rs:1172`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1172) | `present` |
| `enum` | `FixSequenceError` | FIX sequence tracking errors | [`crates/of_fix/src/lib.rs:1209`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1209) | `present` |
| `struct` | `FixSessionId` | Borrowed FIX session identity | [`crates/of_fix/src/lib.rs:1243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1243) | `present` |
| `fn` | `new` | Creates a session id without a qualifier | [`crates/of_fix/src/lib.rs:1256`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1256) | `present` |
| `fn` | `with_qualifier` | Creates a session id with an optional qualifier | [`crates/of_fix/src/lib.rs:1269`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1269) | `present` |
| `fn` | `version` | Returns the FIX version | [`crates/of_fix/src/lib.rs:1287`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1287) | `present` |
| `fn` | `sender_comp_id` | Returns `SenderCompID(49)` | [`crates/of_fix/src/lib.rs:1292`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1292) | `present` |
| `fn` | `target_comp_id` | Returns `TargetCompID(56)` | [`crates/of_fix/src/lib.rs:1297`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1297) | `present` |
| `fn` | `qualifier` | Returns the optional session qualifier bytes | [`crates/of_fix/src/lib.rs:1302`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1302) | `present` |
| `struct` | `FixSequenceSnapshot` | Borrowed persistable sequence-state snapshot | [`crates/of_fix/src/lib.rs:1309`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1309) | `present` |
| `fn` | `new` | Creates a sequence snapshot | [`crates/of_fix/src/lib.rs:1322`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1322) | `present` |
| `fn` | `session_id` | Returns the session id | [`crates/of_fix/src/lib.rs:1338`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1338) | `present` |
| `fn` | `next_inbound` | Returns the next inbound sequence number | [`crates/of_fix/src/lib.rs:1343`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1343) | `present` |
| `fn` | `next_outbound` | Returns the next outbound sequence number | [`crates/of_fix/src/lib.rs:1348`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1348) | `present` |
| `fn` | `trading_day` | Returns the trading day or session date bytes | [`crates/of_fix/src/lib.rs:1353`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1353) | `present` |
| `struct` | `FixOwnedSessionId` | Owned FIX session identity loaded from durable storage | [`crates/of_fix/src/lib.rs:1361`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1361) | `present` |
| `fn` | `new` | Creates an owned session id | [`crates/of_fix/src/lib.rs:1374`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1374) | `present` |
| `fn` | `with_qualifier` | Creates an owned session id with a qualifier | [`crates/of_fix/src/lib.rs:1387`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1387) | `present` |
| `fn` | `from_borrowed` | Creates an owned id from a borrowed session id | [`crates/of_fix/src/lib.rs:1408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1408) | `present` |
| `fn` | `as_borrowed` | Returns a borrowed session id view | [`crates/of_fix/src/lib.rs:1422`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1422) | `present` |
| `fn` | `version` | Returns the FIX version | [`crates/of_fix/src/lib.rs:1432`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1432) | `present` |
| `fn` | `sender_comp_id` | Returns `SenderCompID(49)` | [`crates/of_fix/src/lib.rs:1437`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1437) | `present` |
| `fn` | `target_comp_id` | Returns `TargetCompID(56)` | [`crates/of_fix/src/lib.rs:1442`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1442) | `present` |
| `fn` | `qualifier` | Returns the optional session qualifier | [`crates/of_fix/src/lib.rs:1447`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1447) | `present` |
| `struct` | `FixOwnedSequenceSnapshot` | Owned persistable sequence-state snapshot loaded from storage | [`crates/of_fix/src/lib.rs:1455`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1455) | `present` |
| `fn` | `new` | Creates an owned sequence snapshot | [`crates/of_fix/src/lib.rs:1469`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1469) | `present` |
| `fn` | `from_borrowed` | Creates an owned snapshot from a borrowed sequence snapshot | [`crates/of_fix/src/lib.rs:1489`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1489) | `present` |
| `fn` | `as_borrowed` | Returns a borrowed snapshot view | [`crates/of_fix/src/lib.rs:1506`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1506) | `present` |
| `fn` | `session_id` | Returns the owned session id | [`crates/of_fix/src/lib.rs:1516`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1516) | `present` |
| `fn` | `next_inbound` | Returns the next inbound sequence number | [`crates/of_fix/src/lib.rs:1521`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1521) | `present` |
| `fn` | `next_outbound` | Returns the next outbound sequence number | [`crates/of_fix/src/lib.rs:1526`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1526) | `present` |
| `fn` | `trading_day` | Returns the trading day bytes | [`crates/of_fix/src/lib.rs:1531`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1531) | `present` |
| `fn` | `checksum` | Returns the stored snapshot checksum | [`crates/of_fix/src/lib.rs:1536`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1536) | `present` |
| `fn` | `validate_checksum` | Returns true when the stored checksum matches the snapshot payload | [`crates/of_fix/src/lib.rs:1541`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1541) | `present` |
| `struct` | `FixSequenceStoreConfig` | File-backed FIX sequence snapshot store configuration | [`crates/of_fix/src/lib.rs:1549`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1549) | `present` |
| `fn` | `new` | Creates a sequence store config rooted at `root` | [`crates/of_fix/src/lib.rs:1556`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1556) | `present` |
| `fn` | `with_sync_on_save` | Sets whether snapshot files are synced before atomic rename | [`crates/of_fix/src/lib.rs:1564`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1564) | `present` |
| `fn` | `root` | Returns the sequence snapshot root directory | [`crates/of_fix/src/lib.rs:1570`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1570) | `present` |
| `fn` | `sync_on_save` | Returns whether save operations sync snapshot bytes | [`crates/of_fix/src/lib.rs:1575`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1575) | `present` |
| `struct` | `FixSequenceSnapshotManifest` | Metadata for an installed FIX sequence snapshot | [`crates/of_fix/src/lib.rs:1583`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1583) | `present` |
| `enum` | `FixSequenceStoreError` | Error returned by FIX sequence snapshot persistence | [`crates/of_fix/src/lib.rs:1599`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1599) | `present` |
| `trait` | `FixSequenceSnapshotStore` | FIX sequence snapshot persistence contract | [`crates/of_fix/src/lib.rs:1652`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1652) | `present` |
| `struct` | `FileFixSequenceSnapshotStore` | Atomic file-backed FIX sequence snapshot store | [`crates/of_fix/src/lib.rs:1674`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1674) | `present` |
| `fn` | `open` | Opens or creates a file-backed sequence snapshot store | [`crates/of_fix/src/lib.rs:1684`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1684) | `present` |
| `fn` | `config` | Returns the store configuration | [`crates/of_fix/src/lib.rs:1690`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1690) | `present` |
| `fn` | `snapshot_path` | Returns the latest snapshot path | [`crates/of_fix/src/lib.rs:1695`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1695) | `present` |
| `enum` | `FixSentMessageKind` | Classification for outbound messages retained for resend handling | [`crates/of_fix/src/lib.rs:1755`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1755) | `present` |
| `fn` | `replayable` | Returns whether this message kind is replayable by default | [`crates/of_fix/src/lib.rs:1767`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1767) | `present` |
| `struct` | `FixResendStoreConfig` | Bounded resend-store configuration | [`crates/of_fix/src/lib.rs:1774`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1774) | `present` |
| `fn` | `new` | Creates a bounded resend-store configuration | [`crates/of_fix/src/lib.rs:1784`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1784) | `present` |
| `fn` | `max_messages` | Returns the maximum retained message count | [`crates/of_fix/src/lib.rs:1792`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1792) | `present` |
| `fn` | `max_bytes` | Returns the maximum retained raw-byte count | [`crates/of_fix/src/lib.rs:1797`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1797) | `present` |
| `enum` | `FixResendStoreError` | Resend-store append errors | [`crates/of_fix/src/lib.rs:1814`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1814) | `present` |
| `struct` | `FixDurableResendStoreConfig` | File-backed durable resend-message store configuration | [`crates/of_fix/src/lib.rs:1847`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1847) | `present` |
| `fn` | `new` | Creates a durable resend-store config for `path` | [`crates/of_fix/src/lib.rs:1854`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1854) | `present` |
| `fn` | `with_sync_on_record` | Sets whether each appended resend record is synced before returning | [`crates/of_fix/src/lib.rs:1862`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1862) | `present` |
| `fn` | `path` | Returns the durable resend log path | [`crates/of_fix/src/lib.rs:1868`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1868) | `present` |
| `fn` | `sync_on_record` | Returns whether append operations sync record bytes | [`crates/of_fix/src/lib.rs:1873`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1873) | `present` |
| `struct` | `FixDurableResendAppend` | Metadata for one durable resend append | [`crates/of_fix/src/lib.rs:1881`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1881) | `present` |
| `struct` | `FixDurableResendReplayReport` | Summary produced by replaying durable resend frames | [`crates/of_fix/src/lib.rs:1897`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1897) | `present` |
| `enum` | `FixDurableResendStoreError` | Error returned by durable FIX resend-message persistence | [`crates/of_fix/src/lib.rs:1917`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1917) | `present` |
| `trait` | `FixDurableResendMessageStore` | Durable resend-message persistence contract | [`crates/of_fix/src/lib.rs:2003`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2003) | `present` |
| `struct` | `FileFixDurableResendStore` | Append-only file-backed durable FIX resend-message store | [`crates/of_fix/src/lib.rs:2030`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2030) | `present` |
| `fn` | `open` | Opens or creates an append-only durable resend-message store | [`crates/of_fix/src/lib.rs:2049`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2049) | `present` |
| `fn` | `config` | Returns the durable resend-store configuration | [`crates/of_fix/src/lib.rs:2076`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2076) | `present` |
| `fn` | `inspect_path` | Inspects a durable resend log without opening it for append | [`crates/of_fix/src/lib.rs:2086`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2086) | `present` |
| `struct` | `FixStoredMessage` | Retained outbound FIX frame | [`crates/of_fix/src/lib.rs:2163`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2163) | `present` |
| `fn` | `seq_no` | Returns the outbound `MsgSeqNum(34)` | [`crates/of_fix/src/lib.rs:2171`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2171) | `present` |
| `fn` | `kind` | Returns the retained message kind | [`crates/of_fix/src/lib.rs:2176`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2176) | `present` |
| `fn` | `raw` | Returns the retained raw FIX frame | [`crates/of_fix/src/lib.rs:2181`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2181) | `present` |
| `fn` | `replayable` | Returns whether the message is replayable by default | [`crates/of_fix/src/lib.rs:2186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2186) | `present` |
| `struct` | `FixResendRetention` | Result of recording a sent message into a resend store | [`crates/of_fix/src/lib.rs:2193`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2193) | `present` |
| `fn` | `retained` | Returns whether the message was retained | [`crates/of_fix/src/lib.rs:2201`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2201) | `present` |
| `fn` | `evicted_messages` | Returns messages evicted to satisfy configured bounds | [`crates/of_fix/src/lib.rs:2206`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2206) | `present` |
| `fn` | `evicted_bytes` | Returns bytes evicted to satisfy configured bounds | [`crates/of_fix/src/lib.rs:2211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2211) | `present` |
| `struct` | `FixResendStoreMetrics` | Snapshot of resend-store counters and retained range | [`crates/of_fix/src/lib.rs:2218`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2218) | `present` |
| `fn` | `retained_messages` | Returns the number of retained messages | [`crates/of_fix/src/lib.rs:2231`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2231) | `present` |
| `fn` | `retained_bytes` | Returns the number of retained raw bytes | [`crates/of_fix/src/lib.rs:2236`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2236) | `present` |
| `fn` | `dropped_messages` | Returns messages dropped because retention was disabled or the frame exceeded the byte budget | [`crates/of_fix/src/lib.rs:2242`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2242) | `present` |
| `fn` | `dropped_bytes` | Returns bytes dropped because retention was disabled or the frame exceeded the byte budget | [`crates/of_fix/src/lib.rs:2248`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2248) | `present` |
| `fn` | `evicted_messages` | Returns messages evicted by bounded retention | [`crates/of_fix/src/lib.rs:2253`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2253) | `present` |
| `fn` | `evicted_bytes` | Returns bytes evicted by bounded retention | [`crates/of_fix/src/lib.rs:2258`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2258) | `present` |
| `fn` | `oldest_seq_no` | Returns the oldest retained outbound sequence number | [`crates/of_fix/src/lib.rs:2263`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2263) | `present` |
| `fn` | `newest_seq_no` | Returns the newest observed outbound sequence number | [`crates/of_fix/src/lib.rs:2268`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2268) | `present` |
| `enum` | `FixResendAction` | One planned response for an outbound resend request | [`crates/of_fix/src/lib.rs:2276`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2276) | `present` |
| `struct` | `FixResendPlanSummary` | Summary produced while planning a resend response | [`crates/of_fix/src/lib.rs:2295`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2295) | `present` |
| `fn` | `replay_messages` | Returns replay actions produced by the planner | [`crates/of_fix/src/lib.rs:2303`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2303) | `present` |
| `fn` | `gap_fill_messages` | Returns gap-fill actions produced by the planner | [`crates/of_fix/src/lib.rs:2308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2308) | `present` |
| `fn` | `gap_fill_sequences` | Returns total skipped sequence numbers covered by gap fills | [`crates/of_fix/src/lib.rs:2313`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2313) | `present` |
| `enum` | `FixTranscriptDirection` | Direction of a captured FIX transcript frame | [`crates/of_fix/src/lib.rs:2321`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2321) | `present` |
| `struct` | `FixTranscriptMsgType` | Fixed-size transcript message-type copy | [`crates/of_fix/src/lib.rs:2339`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2339) | `present` |
| `fn` | `empty` | Creates an empty message-type marker | [`crates/of_fix/src/lib.rs:2346`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2346) | `present` |
| `fn` | `new` | Creates a transcript message type from wire bytes | [`crates/of_fix/src/lib.rs:2359`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2359) | `present` |
| `fn` | `as_bytes` | Returns the copied message-type bytes | [`crates/of_fix/src/lib.rs:2375`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2375) | `present` |
| `fn` | `is_empty` | Returns true when no message type was available | [`crates/of_fix/src/lib.rs:2380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2380) | `present` |
| `struct` | `FixTranscriptConfig` | Bounded transcript capture configuration | [`crates/of_fix/src/lib.rs:2393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2393) | `present` |
| `fn` | `new` | Creates a transcript capture configuration | [`crates/of_fix/src/lib.rs:2405`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2405) | `present` |
| `fn` | `max_records` | Returns the maximum retained record count | [`crates/of_fix/src/lib.rs:2414`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2414) | `present` |
| `fn` | `max_raw_bytes` | Returns the maximum retained raw byte count | [`crates/of_fix/src/lib.rs:2419`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2419) | `present` |
| `fn` | `retain_raw` | Returns whether raw FIX frames are retained when they fit | [`crates/of_fix/src/lib.rs:2424`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2424) | `present` |
| `enum` | `FixTranscriptError` | Transcript capture errors | [`crates/of_fix/src/lib.rs:2442`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2442) | `present` |
| `struct` | `FixTranscriptRecord` | Retained transcript frame metadata and optional raw bytes | [`crates/of_fix/src/lib.rs:2467`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2467) | `present` |
| `fn` | `ordinal` | Returns the one-based capture ordinal | [`crates/of_fix/src/lib.rs:2482`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2482) | `present` |
| `fn` | `timestamp_ns` | Returns the caller-provided capture timestamp in nanoseconds | [`crates/of_fix/src/lib.rs:2487`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2487) | `present` |
| `fn` | `direction` | Returns the capture direction | [`crates/of_fix/src/lib.rs:2492`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2492) | `present` |
| `fn` | `seq_no` | Returns `MsgSeqNum(34)` when known | [`crates/of_fix/src/lib.rs:2497`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2497) | `present` |
| `fn` | `msg_type` | Returns the copied `MsgType(35)` bytes when known | [`crates/of_fix/src/lib.rs:2502`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2502) | `present` |
| `fn` | `raw_len` | Returns the original raw frame length | [`crates/of_fix/src/lib.rs:2507`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2507) | `present` |
| `fn` | `raw_checksum` | Returns the FIX modulo-256 checksum over the raw frame bytes | [`crates/of_fix/src/lib.rs:2512`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2512) | `present` |
| `fn` | `raw_hash` | Returns the FNV-1a hash over the raw frame bytes | [`crates/of_fix/src/lib.rs:2517`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2517) | `present` |
| `fn` | `raw_retained` | Returns whether raw frame bytes were retained | [`crates/of_fix/src/lib.rs:2522`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2522) | `present` |
| `fn` | `raw` | Returns retained raw frame bytes, or an empty slice when raw retention was disabled, oversized, or evicted with the record | [`crates/of_fix/src/lib.rs:2528`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2528) | `present` |
| `struct` | `FixTranscriptRetention` | Result of recording a transcript frame | [`crates/of_fix/src/lib.rs:2535`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2535) | `present` |
| `fn` | `retained` | Returns whether the transcript record metadata was retained | [`crates/of_fix/src/lib.rs:2544`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2544) | `present` |
| `fn` | `raw_retained` | Returns whether raw frame bytes were retained | [`crates/of_fix/src/lib.rs:2549`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2549) | `present` |
| `fn` | `evicted_records` | Returns records evicted to satisfy configured bounds | [`crates/of_fix/src/lib.rs:2554`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2554) | `present` |
| `fn` | `evicted_raw_bytes` | Returns retained raw bytes evicted to satisfy configured bounds | [`crates/of_fix/src/lib.rs:2559`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2559) | `present` |
| `struct` | `FixTranscriptMetrics` | Snapshot of transcript capture counters | [`crates/of_fix/src/lib.rs:2566`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2566) | `present` |
| `fn` | `captured_records` | Returns the total number of frames observed by the capture | [`crates/of_fix/src/lib.rs:2581`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2581) | `present` |
| `fn` | `retained_records` | Returns the number of retained transcript records | [`crates/of_fix/src/lib.rs:2586`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2586) | `present` |
| `fn` | `retained_raw_bytes` | Returns retained raw frame bytes | [`crates/of_fix/src/lib.rs:2591`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2591) | `present` |
| `fn` | `dropped_records` | Returns records dropped because record retention was disabled | [`crates/of_fix/src/lib.rs:2596`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2596) | `present` |
| `fn` | `dropped_raw_bytes` | Returns raw bytes not retained because raw retention was disabled, oversized, or record retention was disabled | [`crates/of_fix/src/lib.rs:2602`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2602) | `present` |
| `fn` | `evicted_records` | Returns records evicted by bounded retention | [`crates/of_fix/src/lib.rs:2607`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2607) | `present` |
| `fn` | `evicted_raw_bytes` | Returns raw bytes evicted by bounded retention | [`crates/of_fix/src/lib.rs:2612`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2612) | `present` |
| `fn` | `oldest_ordinal` | Returns the oldest retained transcript ordinal | [`crates/of_fix/src/lib.rs:2617`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2617) | `present` |
| `fn` | `newest_ordinal` | Returns the newest captured transcript ordinal | [`crates/of_fix/src/lib.rs:2622`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2622) | `present` |
| `fn` | `rolling_hash` | Returns the deterministic rolling hash over captured metadata and raw bytes | [`crates/of_fix/src/lib.rs:2628`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2628) | `present` |
| `struct` | `FixTranscriptCapture` | Bounded in-memory FIX transcript capture | [`crates/of_fix/src/lib.rs:2640`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2640) | `present` |
| `fn` | `new` | Creates an empty transcript capture | [`crates/of_fix/src/lib.rs:2660`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2660) | `present` |
| `fn` | `config` | Returns the configured capture bounds | [`crates/of_fix/src/lib.rs:2675`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2675) | `present` |
| `fn` | `records` | Returns retained transcript records in capture order | [`crates/of_fix/src/lib.rs:2680`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2680) | `present` |
| `fn` | `record_message` | Records a parsed validated FIX message | [`crates/of_fix/src/lib.rs:2690`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2690) | `present` |
| `fn` | `record_frame` | Records a raw FIX frame with caller-provided sequence and message type metadata | [`crates/of_fix/src/lib.rs:2712`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2712) | `present` |
| `fn` | `clear_retained` | Clears retained records and byte counters without resetting cumulative capture/drop/eviction counters or the rolling hash | [`crates/of_fix/src/lib.rs:2783`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2783) | `present` |
| `fn` | `metrics` | Returns transcript capture metrics | [`crates/of_fix/src/lib.rs:2789`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2789) | `present` |
| `struct` | `FixResendStore` | Bounded in-memory FIX resend store | [`crates/of_fix/src/lib.rs:2838`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2838) | `present` |
| `fn` | `new` | Creates an empty resend store | [`crates/of_fix/src/lib.rs:2857`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2857) | `present` |
| `fn` | `config` | Returns the configured retention bounds | [`crates/of_fix/src/lib.rs:2871`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2871) | `present` |
| `fn` | `messages` | Returns retained messages in sequence order | [`crates/of_fix/src/lib.rs:2876`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2876) | `present` |
| `fn` | `get` | Returns a retained message by outbound sequence number | [`crates/of_fix/src/lib.rs:2881`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2881) | `present` |
| `fn` | `record_sent` | Records a sent outbound frame | [`crates/of_fix/src/lib.rs:2897`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2897) | `present` |
| `fn` | `plan_resend_range` | Plans replay and gap-fill actions for an inclusive resend range | [`crates/of_fix/src/lib.rs:2965`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2965) | `present` |
| `fn` | `metrics` | Returns resend-store metrics | [`crates/of_fix/src/lib.rs:3034`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3034) | `present` |
| `struct` | `FixSequenceTracker` | Deterministic inbound/outbound FIX sequence tracker | [`crates/of_fix/src/lib.rs:3058`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3058) | `present` |
| `fn` | `new` | Creates a tracker with both inbound and outbound sequence numbers set to `1` | [`crates/of_fix/src/lib.rs:3072`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3072) | `present` |
| `fn` | `from_next` | Creates a tracker from persisted next inbound and outbound values | [`crates/of_fix/src/lib.rs:3082`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3082) | `present` |
| `fn` | `next_inbound` | Returns the next inbound sequence number expected from the counterparty | [`crates/of_fix/src/lib.rs:3090`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3090) | `present` |
| `fn` | `next_outbound` | Returns the next outbound sequence number to assign | [`crates/of_fix/src/lib.rs:3095`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3095) | `present` |
| `fn` | `assign_outbound` | Assigns and advances the next outbound sequence number | [`crates/of_fix/src/lib.rs:3100`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3100) | `present` |
| `fn` | `observe_message` | Observes an inbound message and returns the sequence action | [`crates/of_fix/src/lib.rs:3111`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3111) | `present` |
| `fn` | `observe_inbound` | Observes an inbound sequence number | [`crates/of_fix/src/lib.rs:3126`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3126) | `present` |
| `fn` | `apply_sequence_reset` | Applies `NewSeqNo(36)` as the next expected inbound sequence number | [`crates/of_fix/src/lib.rs:3165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3165) | `present` |
| `fn` | `set_next_inbound` | Sets the next inbound sequence number from trusted persisted state | [`crates/of_fix/src/lib.rs:3180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3180) | `present` |
| `fn` | `set_next_outbound` | Sets the next outbound sequence number from trusted persisted state | [`crates/of_fix/src/lib.rs:3185`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3185) | `present` |
| `fn` | `snapshot` | Creates a persistable snapshot for this tracker | [`crates/of_fix/src/lib.rs:3194`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3194) | `present` |
| `fn` | `from_snapshot` | Restores tracker counters from a sequence snapshot | [`crates/of_fix/src/lib.rs:3208`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3208) | `present` |
| `fn` | `reset_to_one` | Resets both inbound and outbound counters to `1` | [`crates/of_fix/src/lib.rs:3213`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3213) | `present` |
| `struct` | `FixMessageRule` | Validation rule for one FIX message type | [`crates/of_fix/src/lib.rs:3221`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3221) | `present` |
| `fn` | `new` | Creates a validation rule for a message type | [`crates/of_fix/src/lib.rs:3229`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3229) | `present` |
| `fn` | `msg_type` | Returns the message type this rule validates | [`crates/of_fix/src/lib.rs:3242`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3242) | `present` |
| `fn` | `required_tags` | Returns tags required by this rule | [`crates/of_fix/src/lib.rs:3247`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3247) | `present` |
| `fn` | `disallowed_tags` | Returns tags disallowed by this rule | [`crates/of_fix/src/lib.rs:3252`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3252) | `present` |
| `fn` | `validate` | Validates a parsed message against this rule | [`crates/of_fix/src/lib.rs:3262`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3262) | `present` |
| `struct` | `FixDictionary` | Static FIX dictionary/profile used for message-level validation | [`crates/of_fix/src/lib.rs:3288`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3288) | `present` |
| `fn` | `new` | Creates a dictionary for `version` and static message rules | [`crates/of_fix/src/lib.rs:3295`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3295) | `present` |
| `fn` | `version` | Returns the FIX version this dictionary accepts | [`crates/of_fix/src/lib.rs:3300`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3300) | `present` |
| `fn` | `rules` | Returns all message rules | [`crates/of_fix/src/lib.rs:3305`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3305) | `present` |
| `fn` | `rule_for` | Finds a rule by typed message type | [`crates/of_fix/src/lib.rs:3310`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3310) | `present` |
| `fn` | `rule_for_bytes` | Finds a rule by raw `MsgType(35)` bytes | [`crates/of_fix/src/lib.rs:3315`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3315) | `present` |
| `fn` | `validate` | Validates a parsed message against the dictionary | [`crates/of_fix/src/lib.rs:3327`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3327) | `present` |
| `struct` | `FixDecoder` | Stateless FIX decoder facade | [`crates/of_fix/src/lib.rs:3354`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3354) | `present` |
| `fn` | `new` | Creates a decoder facade | [`crates/of_fix/src/lib.rs:3358`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3358) | `present` |
| `fn` | `parse` | Parses and validates a FIX message into caller-provided scratch | [`crates/of_fix/src/lib.rs:3368`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3368) | `present` |
| `struct` | `FixEncoder` | Reusable FIX encoder with an owned output buffer | [`crates/of_fix/src/lib.rs:3379`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3379) | `present` |
| `fn` | `new` | Creates an encoder with an empty buffer | [`crates/of_fix/src/lib.rs:3385`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3385) | `present` |
| `fn` | `with_capacity` | Creates an encoder with preallocated buffer capacity | [`crates/of_fix/src/lib.rs:3390`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3390) | `present` |
| `fn` | `encode` | Encodes into the reusable internal buffer and returns the encoded frame | [`crates/of_fix/src/lib.rs:3402`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3402) | `present` |
| `fn` | `encode_with_repeating_group` | Encodes a message with one flat repeating group appended to `fields` | [`crates/of_fix/src/lib.rs:3423`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3423) | `present` |
| `fn` | `encode_typed` | Encodes a typed version and message type into the reusable buffer | [`crates/of_fix/src/lib.rs:3448`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3448) | `present` |
| `fn` | `buffer` | Returns the current encoded buffer | [`crates/of_fix/src/lib.rs:3458`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3458) | `present` |
| `fn` | `clear` | Clears the internal buffer without releasing capacity | [`crates/of_fix/src/lib.rs:3463`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3463) | `present` |
| `fn` | `into_buffer` | Consumes the encoder and returns the owned buffer | [`crates/of_fix/src/lib.rs:3468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3468) | `present` |
| `struct` | `FixSessionHeader` | Borrowed standard FIX session header fields used by admin builders | [`crates/of_fix/src/lib.rs:3475`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3475) | `present` |
| `fn` | `new` | Creates a standard session header | [`crates/of_fix/src/lib.rs:3484`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3484) | `present` |
| `fn` | `sender_comp_id` | Returns `SenderCompID(49)` | [`crates/of_fix/src/lib.rs:3499`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3499) | `present` |
| `fn` | `target_comp_id` | Returns `TargetCompID(56)` | [`crates/of_fix/src/lib.rs:3504`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3504) | `present` |
| `fn` | `msg_seq_num` | Returns `MsgSeqNum(34)` | [`crates/of_fix/src/lib.rs:3509`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3509) | `present` |
| `fn` | `sending_time` | Returns `SendingTime(52)` | [`crates/of_fix/src/lib.rs:3514`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3514) | `present` |
| `struct` | `FixNewOrderSingle` | Borrowed NewOrderSingle `<D>` request fields | [`crates/of_fix/src/lib.rs:3521`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3521) | `present` |
| `fn` | `new` | Creates a NewOrderSingle request | [`crates/of_fix/src/lib.rs:3536`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3536) | `present` |
| `fn` | `with_account` | Adds `Account(1)` | [`crates/of_fix/src/lib.rs:3559`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3559) | `present` |
| `fn` | `with_price` | Adds `Price(44)` | [`crates/of_fix/src/lib.rs:3565`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3565) | `present` |
| `fn` | `with_stop_px` | Adds `StopPx(99)` | [`crates/of_fix/src/lib.rs:3571`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3571) | `present` |
| `fn` | `with_time_in_force` | Adds `TimeInForce(59)` | [`crates/of_fix/src/lib.rs:3577`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3577) | `present` |
| `struct` | `FixOrderCancelRequest` | Borrowed OrderCancelRequest `<F>` request fields | [`crates/of_fix/src/lib.rs:3585`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3585) | `present` |
| `fn` | `new` | Creates an OrderCancelRequest | [`crates/of_fix/src/lib.rs:3596`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3596) | `present` |
| `fn` | `with_account` | Adds `Account(1)` | [`crates/of_fix/src/lib.rs:3614`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3614) | `present` |
| `struct` | `FixOrderCancelReplaceRequest` | Borrowed OrderCancelReplaceRequest `<G>` request fields | [`crates/of_fix/src/lib.rs:3622`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3622) | `present` |
| `fn` | `new` | Creates an OrderCancelReplaceRequest | [`crates/of_fix/src/lib.rs:3638`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3638) | `present` |
| `fn` | `with_account` | Adds `Account(1)` | [`crates/of_fix/src/lib.rs:3663`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3663) | `present` |
| `fn` | `with_price` | Adds `Price(44)` | [`crates/of_fix/src/lib.rs:3669`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3669) | `present` |
| `fn` | `with_stop_px` | Adds `StopPx(99)` | [`crates/of_fix/src/lib.rs:3675`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3675) | `present` |
| `fn` | `with_time_in_force` | Adds `TimeInForce(59)` | [`crates/of_fix/src/lib.rs:3681`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3681) | `present` |
| `struct` | `FixOrderStatusRequest` | Borrowed OrderStatusRequest `<H>` request fields | [`crates/of_fix/src/lib.rs:3689`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3689) | `present` |
| `fn` | `new` | Creates an OrderStatusRequest | [`crates/of_fix/src/lib.rs:3696`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3696) | `present` |
| `fn` | `with_order_id` | Adds `OrderID(37)` when known | [`crates/of_fix/src/lib.rs:3704`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3704) | `present` |
| `struct` | `FixOrderMassCancelRequest` | Borrowed OrderMassCancelRequest `<q>` request fields | [`crates/of_fix/src/lib.rs:3712`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3712) | `present` |
| `fn` | `new` | Creates an OrderMassCancelRequest | [`crates/of_fix/src/lib.rs:3726`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3726) | `present` |
| `fn` | `with_secondary_cl_ord_id` | Adds `SecondaryClOrdID(526)` | [`crates/of_fix/src/lib.rs:3745`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3745) | `present` |
| `fn` | `with_trading_session_id` | Adds `TradingSessionID(336)` | [`crates/of_fix/src/lib.rs:3751`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3751) | `present` |
| `fn` | `with_trading_session_sub_id` | Adds `TradingSessionSubID(625)` | [`crates/of_fix/src/lib.rs:3757`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3757) | `present` |
| `fn` | `with_symbol` | Adds `Symbol(55)` | [`crates/of_fix/src/lib.rs:3763`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3763) | `present` |
| `fn` | `with_side` | Adds `Side(54)` | [`crates/of_fix/src/lib.rs:3769`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3769) | `present` |
| `fn` | `with_text` | Adds `Text(58)` | [`crates/of_fix/src/lib.rs:3775`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3775) | `present` |
| `struct` | `FixOrderMassStatusRequest` | Borrowed OrderMassStatusRequest `<AF>` request fields | [`crates/of_fix/src/lib.rs:3783`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3783) | `present` |
| `fn` | `new` | Creates an OrderMassStatusRequest | [`crates/of_fix/src/lib.rs:3796`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3796) | `present` |
| `fn` | `with_account` | Adds `Account(1)` | [`crates/of_fix/src/lib.rs:3813`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3813) | `present` |
| `fn` | `with_acct_id_source` | Adds `AcctIDSource(660)` | [`crates/of_fix/src/lib.rs:3819`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3819) | `present` |
| `fn` | `with_trading_session_id` | Adds `TradingSessionID(336)` | [`crates/of_fix/src/lib.rs:3825`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3825) | `present` |
| `fn` | `with_trading_session_sub_id` | Adds `TradingSessionSubID(625)` | [`crates/of_fix/src/lib.rs:3831`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3831) | `present` |
| `fn` | `with_symbol` | Adds `Symbol(55)` | [`crates/of_fix/src/lib.rs:3837`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3837) | `present` |
| `fn` | `with_side` | Adds `Side(54)` | [`crates/of_fix/src/lib.rs:3843`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3843) | `present` |
| `fn` | `parse_message` | Parses and validates a FIX tag-value message into `scratch` | [`crates/of_fix/src/lib.rs:3859`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3859) | `present` |
| `fn` | `parse_session_reject` | Parses a validated Session Reject `<3>` message into a borrowed view | [`crates/of_fix/src/lib.rs:3960`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3960) | `present` |
| `fn` | `parse_business_message_reject` | Parses a validated BusinessMessageReject `<j>` message into a borrowed view | [`crates/of_fix/src/lib.rs:3981`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L3981) | `present` |
| `fn` | `encode_message` | Encodes a FIX tag-value message into `out` | [`crates/of_fix/src/lib.rs:4008`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4008) | `present` |
| `fn` | `encode_message_with_repeating_group` | Encodes a FIX message with one flat repeating group appended to `fields` | [`crates/of_fix/src/lib.rs:4029`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4029) | `present` |
| `fn` | `encode_poss_dup_replay` | Encodes a retained source message as a possible-duplicate resend | [`crates/of_fix/src/lib.rs:4061`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4061) | `present` |
| `fn` | `encode_logon` | Encodes a Logon `<A>` admin message | [`crates/of_fix/src/lib.rs:4127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4127) | `present` |
| `fn` | `encode_heartbeat` | Encodes a Heartbeat `<0>` admin message | [`crates/of_fix/src/lib.rs:4152`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4152) | `present` |
| `fn` | `encode_test_request` | Encodes a TestRequest `<1>` admin message | [`crates/of_fix/src/lib.rs:4179`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4179) | `present` |
| `fn` | `encode_resend_request` | Encodes a ResendRequest `<2>` admin message | [`crates/of_fix/src/lib.rs:4194`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4194) | `present` |
| `fn` | `encode_sequence_reset_gap_fill` | Encodes a SequenceReset `<4>` gap-fill admin message | [`crates/of_fix/src/lib.rs:4216`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4216) | `present` |
| `fn` | `encode_logout` | Encodes a Logout `<5>` admin message | [`crates/of_fix/src/lib.rs:4236`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4236) | `present` |
| `fn` | `encode_new_order_single` | Encodes a NewOrderSingle `<D>` application message | [`crates/of_fix/src/lib.rs:4266`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4266) | `present` |
| `fn` | `encode_order_cancel_request` | Encodes an OrderCancelRequest `<F>` application message | [`crates/of_fix/src/lib.rs:4325`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4325) | `present` |
| `fn` | `encode_order_cancel_replace_request` | Encodes an OrderCancelReplaceRequest `<G>` application message | [`crates/of_fix/src/lib.rs:4367`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4367) | `present` |
| `fn` | `encode_order_status_request` | Encodes an OrderStatusRequest `<H>` application message | [`crates/of_fix/src/lib.rs:4427`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4427) | `present` |
| `fn` | `encode_order_mass_cancel_request` | Encodes an OrderMassCancelRequest `<q>` application message | [`crates/of_fix/src/lib.rs:4456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4456) | `present` |
| `fn` | `encode_order_mass_status_request` | Encodes an OrderMassStatusRequest `<AF>` application message | [`crates/of_fix/src/lib.rs:4515`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4515) | `present` |
| `fn` | `checksum` | Computes a FIX modulo-256 checksum | [`crates/of_fix/src/lib.rs:4719`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L4719) | `present` |
| `fn` | `debug_render` | Renders a FIX frame with `\|` in place of SOH | [`crates/of_fix/src/lib.rs:5147`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L5147) | `present` |
| `struct` | `FixSessionEngineConfig` | Configuration for a deterministic FIX session engine | [`crates/of_fix/src/session.rs:24`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L24) | `present` |
| `fn` | `new` | Creates a session configuration from the negotiated heartbeat interval | [`crates/of_fix/src/session.rs:46`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L46) | `present` |
| `fn` | `with_timeouts` | Overrides liveness and logout durations in nanoseconds | [`crates/of_fix/src/session.rs:78`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L78) | `present` |
| `fn` | `with_reset_seq_num_on_logon` | Configures whether the next connection sends `ResetSeqNumFlag(141)=Y` | [`crates/of_fix/src/session.rs:104`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L104) | `present` |
| `fn` | `with_comp_id_validation` | Configures strict sender/target component-id validation | [`crates/of_fix/src/session.rs:113`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L113) | `present` |
| `fn` | `heartbeat_interval_secs` | Returns the negotiated `HeartBtInt(108)` value in seconds | [`crates/of_fix/src/session.rs:119`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L119) | `present` |
| `fn` | `heartbeat_interval_ns` | Returns the outbound-heartbeat idle duration in nanoseconds | [`crates/of_fix/src/session.rs:124`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L124) | `present` |
| `fn` | `test_request_after_ns` | Returns the inbound-idle duration before a TestRequest is sent | [`crates/of_fix/src/session.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L129) | `present` |
| `fn` | `disconnect_after_test_request_ns` | Returns the unanswered-TestRequest duration before disconnect is requested | [`crates/of_fix/src/session.rs:135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L135) | `present` |
| `fn` | `logout_timeout_ns` | Returns the Logout response timeout in nanoseconds | [`crates/of_fix/src/session.rs:140`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L140) | `present` |
| `fn` | `reset_seq_num_on_logon` | Returns whether Logon requests a bilateral sequence reset | [`crates/of_fix/src/session.rs:145`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L145) | `present` |
| `fn` | `validate_comp_ids` | Returns whether inbound component identifiers are validated | [`crates/of_fix/src/session.rs:150`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L150) | `present` |
| `enum` | `FixSessionConfigError` | Invalid FIX session-engine configuration | [`crates/of_fix/src/session.rs:158`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L158) | `present` |
| `enum` | `FixSessionSendKind` | Administrative message kind emitted into the caller-owned output buffer | [`crates/of_fix/src/session.rs:192`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L192) | `present` |
| `enum` | `FixSessionDisconnectReason` | Reason the session asks its host to close the transport | [`crates/of_fix/src/session.rs:208`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L208) | `present` |
| `enum` | `FixSessionAction` | Deterministic action produced by a FIX session-engine call | [`crates/of_fix/src/session.rs:224`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L224) | `present` |
| `struct` | `FixSessionMetrics` | Allocation-free FIX session counters and timing snapshot | [`crates/of_fix/src/session.rs:292`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L292) | `present` |
| `enum` | `FixSessionError` | FIX session protocol and state-machine errors | [`crates/of_fix/src/session.rs:342`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L342) | `present` |
| `struct` | `FixSessionEngine` | Single-owner deterministic FIX session state machine | [`crates/of_fix/src/session.rs:490`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L490) | `present` |
| `fn` | `new` | Creates a disconnected session with sequence numbers starting at one | [`crates/of_fix/src/session.rs:506`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L506) | `present` |
| `fn` | `with_sequences` | Creates a disconnected session from restored sequence counters | [`crates/of_fix/src/session.rs:511`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L511) | `present` |
| `fn` | `config` | Returns immutable session configuration | [`crates/of_fix/src/session.rs:532`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L532) | `present` |
| `fn` | `session_id` | Returns the owned session identity | [`crates/of_fix/src/session.rs:537`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L537) | `present` |
| `fn` | `state` | Returns current lifecycle state | [`crates/of_fix/src/session.rs:542`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L542) | `present` |
| `fn` | `sequences` | Returns current sequence counters | [`crates/of_fix/src/session.rs:547`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L547) | `present` |
| `fn` | `metrics` | Returns an allocation-free metrics snapshot | [`crates/of_fix/src/session.rs:552`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L552) | `present` |
| `fn` | `on_transport_connecting` | Marks the beginning of a transport connection attempt | [`crates/of_fix/src/session.rs:562`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L562) | `present` |
| `fn` | `on_transport_connected` | Encodes Logon after the transport has connected | [`crates/of_fix/src/session.rs:585`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L585) | `present` |
| `fn` | `on_transport_disconnected` | Records that the transport closed | [`crates/of_fix/src/session.rs:637`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L637) | `present` |
| `fn` | `stop` | Stops the session without emitting Logout | [`crates/of_fix/src/session.rs:653`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L653) | `present` |
| `fn` | `request_logout` | Encodes a graceful Logout request | [`crates/of_fix/src/session.rs:667`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L667) | `present` |
| `fn` | `on_timer` | Runs deterministic heartbeat, TestRequest, and Logout timers | [`crates/of_fix/src/session.rs:717`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L717) | `present` |
| `fn` | `on_inbound` | Processes one already-decoded inbound FIX frame | [`crates/of_fix/src/session.rs:800`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L800) | `present` |
| `fn` | `assign_application_sequence` | Assigns the next outbound application sequence number | [`crates/of_fix/src/session.rs:900`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L900) | `present` |
| `fn` | `record_replay_sent` | Records transmission of a possible-duplicate replay frame | [`crates/of_fix/src/session.rs:921`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L921) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `const` | `-` | `SOH` | `: u8 = 0x01` | [`crates/of_fix/src/lib.rs:25`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L25) |
| `const` | `-` | `BEGIN_STRING` | `: Self = Self(8)` | [`crates/of_fix/src/lib.rs:33`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L33) |
| `const` | `-` | `ACCOUNT` | `: Self = Self(1)` | [`crates/of_fix/src/lib.rs:35`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L35) |
| `const` | `-` | `BODY_LENGTH` | `: Self = Self(9)` | [`crates/of_fix/src/lib.rs:37`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L37) |
| `const` | `-` | `BEGIN_SEQ_NO` | `: Self = Self(7)` | [`crates/of_fix/src/lib.rs:39`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L39) |
| `const` | `-` | `END_SEQ_NO` | `: Self = Self(16)` | [`crates/of_fix/src/lib.rs:41`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L41) |
| `const` | `-` | `MSG_TYPE` | `: Self = Self(35)` | [`crates/of_fix/src/lib.rs:43`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L43) |
| `const` | `-` | `MSG_SEQ_NUM` | `: Self = Self(34)` | [`crates/of_fix/src/lib.rs:45`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L45) |
| `const` | `-` | `NEW_SEQ_NO` | `: Self = Self(36)` | [`crates/of_fix/src/lib.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L47) |
| `const` | `-` | `POSS_DUP_FLAG` | `: Self = Self(43)` | [`crates/of_fix/src/lib.rs:49`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L49) |
| `const` | `-` | `REF_SEQ_NUM` | `: Self = Self(45)` | [`crates/of_fix/src/lib.rs:51`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L51) |
| `const` | `-` | `SENDER_COMP_ID` | `: Self = Self(49)` | [`crates/of_fix/src/lib.rs:53`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L53) |
| `const` | `-` | `SENDING_TIME` | `: Self = Self(52)` | [`crates/of_fix/src/lib.rs:55`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L55) |
| `const` | `-` | `TARGET_COMP_ID` | `: Self = Self(56)` | [`crates/of_fix/src/lib.rs:57`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L57) |
| `const` | `-` | `CL_ORD_ID` | `: Self = Self(11)` | [`crates/of_fix/src/lib.rs:59`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L59) |
| `const` | `-` | `ORIG_CL_ORD_ID` | `: Self = Self(41)` | [`crates/of_fix/src/lib.rs:61`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L61) |
| `const` | `-` | `ORDER_ID` | `: Self = Self(37)` | [`crates/of_fix/src/lib.rs:63`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L63) |
| `const` | `-` | `EXEC_ID` | `: Self = Self(17)` | [`crates/of_fix/src/lib.rs:65`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L65) |
| `const` | `-` | `EXEC_TYPE` | `: Self = Self(150)` | [`crates/of_fix/src/lib.rs:67`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L67) |
| `const` | `-` | `ORD_STATUS` | `: Self = Self(39)` | [`crates/of_fix/src/lib.rs:69`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L69) |
| `const` | `-` | `SYMBOL` | `: Self = Self(55)` | [`crates/of_fix/src/lib.rs:71`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L71) |
| `const` | `-` | `SIDE` | `: Self = Self(54)` | [`crates/of_fix/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L73) |
| `const` | `-` | `TRADING_SESSION_ID` | `: Self = Self(336)` | [`crates/of_fix/src/lib.rs:75`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L75) |
| `const` | `-` | `ENCODED_TEXT_LEN` | `: Self = Self(354)` | [`crates/of_fix/src/lib.rs:77`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L77) |
| `const` | `-` | `ORDER_QTY` | `: Self = Self(38)` | [`crates/of_fix/src/lib.rs:79`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L79) |
| `const` | `-` | `ORD_TYPE` | `: Self = Self(40)` | [`crates/of_fix/src/lib.rs:81`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L81) |
| `const` | `-` | `PRICE` | `: Self = Self(44)` | [`crates/of_fix/src/lib.rs:83`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L83) |
| `const` | `-` | `TIME_IN_FORCE` | `: Self = Self(59)` | [`crates/of_fix/src/lib.rs:85`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L85) |
| `const` | `-` | `STOP_PX` | `: Self = Self(99)` | [`crates/of_fix/src/lib.rs:87`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L87) |
| `const` | `-` | `LAST_QTY` | `: Self = Self(32)` | [`crates/of_fix/src/lib.rs:89`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L89) |
| `const` | `-` | `LAST_PX` | `: Self = Self(31)` | [`crates/of_fix/src/lib.rs:91`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L91) |
| `const` | `-` | `CUM_QTY` | `: Self = Self(14)` | [`crates/of_fix/src/lib.rs:93`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L93) |
| `const` | `-` | `LEAVES_QTY` | `: Self = Self(151)` | [`crates/of_fix/src/lib.rs:95`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L95) |
| `const` | `-` | `AVG_PX` | `: Self = Self(6)` | [`crates/of_fix/src/lib.rs:97`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L97) |
| `const` | `-` | `TRANSACT_TIME` | `: Self = Self(60)` | [`crates/of_fix/src/lib.rs:99`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L99) |
| `const` | `-` | `TEXT` | `: Self = Self(58)` | [`crates/of_fix/src/lib.rs:101`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L101) |
| `const` | `-` | `ENCRYPT_METHOD` | `: Self = Self(98)` | [`crates/of_fix/src/lib.rs:103`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L103) |
| `const` | `-` | `TEST_REQ_ID` | `: Self = Self(112)` | [`crates/of_fix/src/lib.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L105) |
| `const` | `-` | `ORIG_SENDING_TIME` | `: Self = Self(122)` | [`crates/of_fix/src/lib.rs:107`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L107) |
| `const` | `-` | `HEART_BT_INT` | `: Self = Self(108)` | [`crates/of_fix/src/lib.rs:109`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L109) |
| `const` | `-` | `GAP_FILL_FLAG` | `: Self = Self(123)` | [`crates/of_fix/src/lib.rs:111`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L111) |
| `const` | `-` | `RESET_SEQ_NUM_FLAG` | `: Self = Self(141)` | [`crates/of_fix/src/lib.rs:113`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L113) |
| `const` | `-` | `REF_TAG_ID` | `: Self = Self(371)` | [`crates/of_fix/src/lib.rs:115`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L115) |
| `const` | `-` | `REF_MSG_TYPE` | `: Self = Self(372)` | [`crates/of_fix/src/lib.rs:117`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L117) |
| `const` | `-` | `SESSION_REJECT_REASON` | `: Self = Self(373)` | [`crates/of_fix/src/lib.rs:119`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L119) |
| `const` | `-` | `BUSINESS_REJECT_REF_ID` | `: Self = Self(379)` | [`crates/of_fix/src/lib.rs:121`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L121) |
| `const` | `-` | `BUSINESS_REJECT_REASON` | `: Self = Self(380)` | [`crates/of_fix/src/lib.rs:123`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L123) |
| `const` | `-` | `SECONDARY_CL_ORD_ID` | `: Self = Self(526)` | [`crates/of_fix/src/lib.rs:125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L125) |
| `const` | `-` | `MASS_CANCEL_REQUEST_TYPE` | `: Self = Self(530)` | [`crates/of_fix/src/lib.rs:127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L127) |
| `const` | `-` | `MASS_STATUS_REQ_ID` | `: Self = Self(584)` | [`crates/of_fix/src/lib.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L129) |
| `const` | `-` | `MASS_STATUS_REQ_TYPE` | `: Self = Self(585)` | [`crates/of_fix/src/lib.rs:131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L131) |
| `const` | `-` | `TRADING_SESSION_SUB_ID` | `: Self = Self(625)` | [`crates/of_fix/src/lib.rs:133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L133) |
| `const` | `-` | `ACCT_ID_SOURCE` | `: Self = Self(660)` | [`crates/of_fix/src/lib.rs:135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L135) |
| `const` | `-` | `CHECK_SUM` | `: Self = Self(10)` | [`crates/of_fix/src/lib.rs:137`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L137) |
| `variant` | `FixVersion` | `Fix40` | `Fix40` | [`crates/of_fix/src/lib.rs:151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L151) |
| `variant` | `FixVersion` | `Fix41` | `Fix41` | [`crates/of_fix/src/lib.rs:153`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L153) |
| `variant` | `FixVersion` | `Fix42` | `Fix42` | [`crates/of_fix/src/lib.rs:155`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L155) |
| `variant` | `FixVersion` | `Fix43` | `Fix43` | [`crates/of_fix/src/lib.rs:157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L157) |
| `variant` | `FixVersion` | `Fix44` | `Fix44` | [`crates/of_fix/src/lib.rs:159`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L159) |
| `variant` | `FixVersion` | `FixT11` | `FixT11` | [`crates/of_fix/src/lib.rs:161`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L161) |
| `const` | `-` | `HEARTBEAT` | `: Self = Self(b"0")` | [`crates/of_fix/src/lib.rs:207`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L207) |
| `const` | `-` | `TEST_REQUEST` | `: Self = Self(b"1")` | [`crates/of_fix/src/lib.rs:209`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L209) |
| `const` | `-` | `RESEND_REQUEST` | `: Self = Self(b"2")` | [`crates/of_fix/src/lib.rs:211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L211) |
| `const` | `-` | `REJECT` | `: Self = Self(b"3")` | [`crates/of_fix/src/lib.rs:213`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L213) |
| `const` | `-` | `SEQUENCE_RESET` | `: Self = Self(b"4")` | [`crates/of_fix/src/lib.rs:215`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L215) |
| `const` | `-` | `LOGOUT` | `: Self = Self(b"5")` | [`crates/of_fix/src/lib.rs:217`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L217) |
| `const` | `-` | `EXECUTION_REPORT` | `: Self = Self(b"8")` | [`crates/of_fix/src/lib.rs:219`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L219) |
| `const` | `-` | `ORDER_CANCEL_REJECT` | `: Self = Self(b"9")` | [`crates/of_fix/src/lib.rs:221`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L221) |
| `const` | `-` | `LOGON` | `: Self = Self(b"A")` | [`crates/of_fix/src/lib.rs:223`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L223) |
| `const` | `-` | `NEW_ORDER_SINGLE` | `: Self = Self(b"D")` | [`crates/of_fix/src/lib.rs:225`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L225) |
| `const` | `-` | `ORDER_CANCEL_REQUEST` | `: Self = Self(b"F")` | [`crates/of_fix/src/lib.rs:227`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L227) |
| `const` | `-` | `ORDER_CANCEL_REPLACE_REQUEST` | `: Self = Self(b"G")` | [`crates/of_fix/src/lib.rs:229`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L229) |
| `const` | `-` | `ORDER_STATUS_REQUEST` | `: Self = Self(b"H")` | [`crates/of_fix/src/lib.rs:231`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L231) |
| `const` | `-` | `BUSINESS_MESSAGE_REJECT` | `: Self = Self(b"j")` | [`crates/of_fix/src/lib.rs:233`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L233) |
| `const` | `-` | `ORDER_MASS_CANCEL_REQUEST` | `: Self = Self(b"q")` | [`crates/of_fix/src/lib.rs:235`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L235) |
| `const` | `-` | `ORDER_MASS_STATUS_REQUEST` | `: Self = Self(b"AF")` | [`crates/of_fix/src/lib.rs:237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L237) |
| `variant` | `FixOrderSide` | `Buy` | `Buy` | [`crates/of_fix/src/lib.rs:310`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L310) |
| `variant` | `FixOrderSide` | `Sell` | `Sell` | [`crates/of_fix/src/lib.rs:312`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L312) |
| `variant` | `FixOrderSide` | `SellShort` | `SellShort` | [`crates/of_fix/src/lib.rs:314`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L314) |
| `variant` | `FixOrdType` | `Market` | `Market` | [`crates/of_fix/src/lib.rs:343`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L343) |
| `variant` | `FixOrdType` | `Limit` | `Limit` | [`crates/of_fix/src/lib.rs:345`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L345) |
| `variant` | `FixOrdType` | `Stop` | `Stop` | [`crates/of_fix/src/lib.rs:347`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L347) |
| `variant` | `FixOrdType` | `StopLimit` | `StopLimit` | [`crates/of_fix/src/lib.rs:349`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L349) |
| `variant` | `FixTimeInForce` | `Day` | `Day` | [`crates/of_fix/src/lib.rs:380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L380) |
| `variant` | `FixTimeInForce` | `GoodTillCancel` | `GoodTillCancel` | [`crates/of_fix/src/lib.rs:382`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L382) |
| `variant` | `FixTimeInForce` | `ImmediateOrCancel` | `ImmediateOrCancel` | [`crates/of_fix/src/lib.rs:384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L384) |
| `variant` | `FixTimeInForce` | `FillOrKill` | `FillOrKill` | [`crates/of_fix/src/lib.rs:386`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L386) |
| `variant` | `FixTimeInForce` | `GoodTillDate` | `GoodTillDate` | [`crates/of_fix/src/lib.rs:388`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L388) |
| `variant` | `FixMassCancelRequestType` | `Security` | `Security` | [`crates/of_fix/src/lib.rs:421`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L421) |
| `variant` | `FixMassCancelRequestType` | `UnderlyingSecurity` | `UnderlyingSecurity` | [`crates/of_fix/src/lib.rs:423`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L423) |
| `variant` | `FixMassCancelRequestType` | `Product` | `Product` | [`crates/of_fix/src/lib.rs:425`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L425) |
| `variant` | `FixMassCancelRequestType` | `CfiCode` | `CfiCode` | [`crates/of_fix/src/lib.rs:427`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L427) |
| `variant` | `FixMassCancelRequestType` | `SecurityType` | `SecurityType` | [`crates/of_fix/src/lib.rs:429`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L429) |
| `variant` | `FixMassCancelRequestType` | `TradingSession` | `TradingSession` | [`crates/of_fix/src/lib.rs:431`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L431) |
| `variant` | `FixMassCancelRequestType` | `AllOrders` | `AllOrders` | [`crates/of_fix/src/lib.rs:433`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L433) |
| `variant` | `FixMassStatusReqType` | `Security` | `Security` | [`crates/of_fix/src/lib.rs:456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L456) |
| `variant` | `FixMassStatusReqType` | `UnderlyingSecurity` | `UnderlyingSecurity` | [`crates/of_fix/src/lib.rs:458`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L458) |
| `variant` | `FixMassStatusReqType` | `Product` | `Product` | [`crates/of_fix/src/lib.rs:460`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L460) |
| `variant` | `FixMassStatusReqType` | `CfiCode` | `CfiCode` | [`crates/of_fix/src/lib.rs:462`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L462) |
| `variant` | `FixMassStatusReqType` | `SecurityType` | `SecurityType` | [`crates/of_fix/src/lib.rs:464`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L464) |
| `variant` | `FixMassStatusReqType` | `TradingSession` | `TradingSession` | [`crates/of_fix/src/lib.rs:466`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L466) |
| `variant` | `FixMassStatusReqType` | `AllOrders` | `AllOrders` | [`crates/of_fix/src/lib.rs:468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L468) |
| `variant` | `FixMassStatusReqType` | `PartyId` | `PartyId` | [`crates/of_fix/src/lib.rs:470`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L470) |
| `field` | `FixFieldView` | `tag` | `: FixTag` | [`crates/of_fix/src/lib.rs:493`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L493) |
| `field` | `FixFieldView` | `value` | `: &'a [u8]` | [`crates/of_fix/src/lib.rs:495`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L495) |
| `variant` | `FixGroupError` | `MissingCountTag` | `MissingCountTag(FixTag)` | [`crates/of_fix/src/lib.rs:786`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L786) |
| `variant` | `FixGroupError` | `InvalidCount` | `InvalidCount(FixTag)` | [`crates/of_fix/src/lib.rs:788`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L788) |
| `variant` | `FixParseError` | `Empty` | `Empty` | [`crates/of_fix/src/lib.rs:846`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L846) |
| `variant` | `FixParseError` | `MalformedField` | `MalformedField` | [`crates/of_fix/src/lib.rs:848`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L848) |
| `variant` | `FixParseError` | `InvalidTag` | `InvalidTag` | [`crates/of_fix/src/lib.rs:850`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L850) |
| `variant` | `FixParseError` | `MissingRequiredTag` | `MissingRequiredTag(FixTag)` | [`crates/of_fix/src/lib.rs:852`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L852) |
| `variant` | `FixParseError` | `InvalidBodyLength` | `InvalidBodyLength` | [`crates/of_fix/src/lib.rs:861`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L861) |
| `variant` | `FixParseError` | `InvalidChecksum` | `InvalidChecksum` | [`crates/of_fix/src/lib.rs:870`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L870) |
| `variant` | `FixEncodeError` | `ValueContainsSoh` | `ValueContainsSoh(FixTag)` | [`crates/of_fix/src/lib.rs:912`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L912) |
| `variant` | `FixEncodeError` | `ReservedTag` | `ReservedTag(FixTag)` | [`crates/of_fix/src/lib.rs:914`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L914) |
| `variant` | `FixEncodeError` | `MissingRequiredTag` | `MissingRequiredTag(FixTag)` | [`crates/of_fix/src/lib.rs:916`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L916) |
| `variant` | `FixEncodeError` | `DuplicateRepeatingGroupCountTag` | `DuplicateRepeatingGroupCountTag(FixTag)` | [`crates/of_fix/src/lib.rs:918`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L918) |
| `variant` | `FixProfileError` | `MissingBeginString` | `MissingBeginString` | [`crates/of_fix/src/lib.rs:981`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L981) |
| `variant` | `FixProfileError` | `UnsupportedVersion` | `UnsupportedVersion` | [`crates/of_fix/src/lib.rs:984`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L984) |
| `variant` | `FixProfileError` | `MissingMsgType` | `MissingMsgType` | [`crates/of_fix/src/lib.rs:993`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L993) |
| `variant` | `FixProfileError` | `UnsupportedMsgType` | `UnsupportedMsgType` | [`crates/of_fix/src/lib.rs:995`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L995) |
| `variant` | `FixRejectParseError` | `InvalidMsgType` | `InvalidMsgType` | [`crates/of_fix/src/lib.rs:1042`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1042) |
| `variant` | `FixRejectParseError` | `MissingTag` | `MissingTag(FixTag)` | [`crates/of_fix/src/lib.rs:1044`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1044) |
| `variant` | `FixRejectParseError` | `InvalidNumber` | `InvalidNumber(FixTag)` | [`crates/of_fix/src/lib.rs:1046`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1046) |
| `variant` | `FixSessionState` | `Disconnected` | `Disconnected` | [`crates/of_fix/src/lib.rs:1140`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1140) |
| `variant` | `FixSessionState` | `Connecting` | `Connecting` | [`crates/of_fix/src/lib.rs:1142`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1142) |
| `variant` | `FixSessionState` | `LogonSent` | `LogonSent` | [`crates/of_fix/src/lib.rs:1144`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1144) |
| `variant` | `FixSessionState` | `Ready` | `Ready` | [`crates/of_fix/src/lib.rs:1146`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1146) |
| `variant` | `FixSessionState` | `ResendRequested` | `ResendRequested` | [`crates/of_fix/src/lib.rs:1149`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1149) |
| `variant` | `FixSessionState` | `Recovering` | `Recovering` | [`crates/of_fix/src/lib.rs:1151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1151) |
| `variant` | `FixSessionState` | `LogoutSent` | `LogoutSent` | [`crates/of_fix/src/lib.rs:1153`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1153) |
| `variant` | `FixSessionState` | `Stopped` | `Stopped` | [`crates/of_fix/src/lib.rs:1155`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1155) |
| `variant` | `FixSessionState` | `Degraded` | `Degraded` | [`crates/of_fix/src/lib.rs:1157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1157) |
| `field` | `FixResendRange` | `begin_seq_no` | `: u64` | [`crates/of_fix/src/lib.rs:1164`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1164) |
| `field` | `FixResendRange` | `end_seq_no` | `: u64` | [`crates/of_fix/src/lib.rs:1166`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1166) |
| `variant` | `FixSequenceError` | `MissingMsgSeqNum` | `MissingMsgSeqNum` | [`crates/of_fix/src/lib.rs:1211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1211) |
| `variant` | `FixSequenceError` | `ZeroSeqNo` | `ZeroSeqNo` | [`crates/of_fix/src/lib.rs:1213`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1213) |
| `field` | `FixSequenceSnapshotManifest` | `path` | `: PathBuf` | [`crates/of_fix/src/lib.rs:1585`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1585) |
| `field` | `FixSequenceSnapshotManifest` | `bytes` | `: u64` | [`crates/of_fix/src/lib.rs:1587`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1587) |
| `field` | `FixSequenceSnapshotManifest` | `checksum` | `: u64` | [`crates/of_fix/src/lib.rs:1589`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1589) |
| `field` | `FixSequenceSnapshotManifest` | `next_inbound` | `: u64` | [`crates/of_fix/src/lib.rs:1591`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1591) |
| `field` | `FixSequenceSnapshotManifest` | `next_outbound` | `: u64` | [`crates/of_fix/src/lib.rs:1593`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1593) |
| `variant` | `FixSequenceStoreError` | `Io` | `Io(String)` | [`crates/of_fix/src/lib.rs:1601`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1601) |
| `variant` | `FixSequenceStoreError` | `Encode` | `Encode(FixEncodeError)` | [`crates/of_fix/src/lib.rs:1603`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1603) |
| `variant` | `FixSequenceStoreError` | `InvalidMagic` | `InvalidMagic` | [`crates/of_fix/src/lib.rs:1605`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1605) |
| `variant` | `FixSequenceStoreError` | `UnsupportedVersion` | `UnsupportedVersion(u16)` | [`crates/of_fix/src/lib.rs:1607`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1607) |
| `variant` | `FixSequenceStoreError` | `Truncated` | `Truncated` | [`crates/of_fix/src/lib.rs:1609`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1609) |
| `variant` | `FixSequenceStoreError` | `InvalidVersion` | `InvalidVersion` | [`crates/of_fix/src/lib.rs:1611`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1611) |
| `variant` | `FixSequenceStoreError` | `FieldTooLarge` | `FieldTooLarge` | [`crates/of_fix/src/lib.rs:1613`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1613) |
| `variant` | `FixSentMessageKind` | `Application` | `Application` | [`crates/of_fix/src/lib.rs:1757`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1757) |
| `variant` | `FixSentMessageKind` | `Administrative` | `Administrative` | [`crates/of_fix/src/lib.rs:1759`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1759) |
| `variant` | `FixSentMessageKind` | `Reject` | `Reject` | [`crates/of_fix/src/lib.rs:1762`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1762) |
| `variant` | `FixResendStoreError` | `ZeroSeqNo` | `ZeroSeqNo` | [`crates/of_fix/src/lib.rs:1816`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1816) |
| `field` | `FixDurableResendAppend` | `seq_no` | `: u64` | [`crates/of_fix/src/lib.rs:1883`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1883) |
| `field` | `FixDurableResendAppend` | `kind` | `: FixSentMessageKind` | [`crates/of_fix/src/lib.rs:1885`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1885) |
| `field` | `FixDurableResendAppend` | `offset` | `: u64` | [`crates/of_fix/src/lib.rs:1887`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1887) |
| `field` | `FixDurableResendAppend` | `bytes` | `: u64` | [`crates/of_fix/src/lib.rs:1889`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1889) |
| `field` | `FixDurableResendAppend` | `checksum` | `: u64` | [`crates/of_fix/src/lib.rs:1891`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1891) |
| `field` | `FixDurableResendReplayReport` | `records` | `: u64` | [`crates/of_fix/src/lib.rs:1899`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1899) |
| `field` | `FixDurableResendReplayReport` | `bytes` | `: u64` | [`crates/of_fix/src/lib.rs:1901`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1901) |
| `field` | `FixDurableResendReplayReport` | `first_seq_no` | `: Option<u64>` | [`crates/of_fix/src/lib.rs:1903`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1903) |
| `field` | `FixDurableResendReplayReport` | `last_seq_no` | `: Option<u64>` | [`crates/of_fix/src/lib.rs:1905`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1905) |
| `field` | `FixDurableResendReplayReport` | `checksum` | `: u64` | [`crates/of_fix/src/lib.rs:1907`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1907) |
| `field` | `FixDurableResendReplayReport` | `retained_messages` | `: u64` | [`crates/of_fix/src/lib.rs:1909`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1909) |
| `field` | `FixDurableResendReplayReport` | `dropped_messages` | `: u64` | [`crates/of_fix/src/lib.rs:1911`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1911) |
| `variant` | `FixDurableResendStoreError` | `Io` | `Io(String)` | [`crates/of_fix/src/lib.rs:1919`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1919) |
| `variant` | `FixDurableResendStoreError` | `ResendStore` | `ResendStore(FixResendStoreError)` | [`crates/of_fix/src/lib.rs:1921`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1921) |
| `variant` | `FixDurableResendStoreError` | `InvalidMagic` | `InvalidMagic` | [`crates/of_fix/src/lib.rs:1923`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1923) |
| `variant` | `FixDurableResendStoreError` | `UnsupportedVersion` | `UnsupportedVersion(u16)` | [`crates/of_fix/src/lib.rs:1925`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1925) |
| `variant` | `FixDurableResendStoreError` | `Truncated` | `Truncated` | [`crates/of_fix/src/lib.rs:1927`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1927) |
| `variant` | `FixDurableResendStoreError` | `InvalidKind` | `InvalidKind(u8)` | [`crates/of_fix/src/lib.rs:1929`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1929) |
| `variant` | `FixDurableResendStoreError` | `FrameTooLarge` | `FrameTooLarge` | [`crates/of_fix/src/lib.rs:1931`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L1931) |
| `variant` | `FixTranscriptDirection` | `Inbound` | `Inbound` | [`crates/of_fix/src/lib.rs:2323`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2323) |
| `variant` | `FixTranscriptDirection` | `Outbound` | `Outbound` | [`crates/of_fix/src/lib.rs:2325`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/lib.rs#L2325) |
| `variant` | `FixSessionConfigError` | `ZeroHeartbeatInterval` | `ZeroHeartbeatInterval` | [`crates/of_fix/src/session.rs:160`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L160) |
| `variant` | `FixSessionConfigError` | `ZeroTestRequestTimeout` | `ZeroTestRequestTimeout` | [`crates/of_fix/src/session.rs:162`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L162) |
| `variant` | `FixSessionConfigError` | `ZeroDisconnectTimeout` | `ZeroDisconnectTimeout` | [`crates/of_fix/src/session.rs:164`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L164) |
| `variant` | `FixSessionConfigError` | `ZeroLogoutTimeout` | `ZeroLogoutTimeout` | [`crates/of_fix/src/session.rs:166`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L166) |
| `variant` | `FixSessionConfigError` | `DurationOverflow` | `DurationOverflow` | [`crates/of_fix/src/session.rs:168`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L168) |
| `variant` | `FixSessionSendKind` | `Logon` | `Logon` | [`crates/of_fix/src/session.rs:194`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L194) |
| `variant` | `FixSessionSendKind` | `Heartbeat` | `Heartbeat` | [`crates/of_fix/src/session.rs:196`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L196) |
| `variant` | `FixSessionSendKind` | `TestRequest` | `TestRequest` | [`crates/of_fix/src/session.rs:198`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L198) |
| `variant` | `FixSessionSendKind` | `ResendRequest` | `ResendRequest` | [`crates/of_fix/src/session.rs:200`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L200) |
| `variant` | `FixSessionSendKind` | `Logout` | `Logout` | [`crates/of_fix/src/session.rs:202`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L202) |
| `variant` | `FixSessionDisconnectReason` | `HeartbeatTimeout` | `HeartbeatTimeout` | [`crates/of_fix/src/session.rs:210`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L210) |
| `variant` | `FixSessionDisconnectReason` | `LogoutTimeout` | `LogoutTimeout` | [`crates/of_fix/src/session.rs:212`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L212) |
| `variant` | `FixSessionDisconnectReason` | `PeerLogout` | `PeerLogout` | [`crates/of_fix/src/session.rs:214`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L214) |
| `variant` | `FixSessionAction` | `None` | `None` | [`crates/of_fix/src/session.rs:226`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L226) |
| `field` | `FixSessionMetrics` | `connections` | `: u64` | [`crates/of_fix/src/session.rs:294`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L294) |
| `field` | `FixSessionMetrics` | `disconnects` | `: u64` | [`crates/of_fix/src/session.rs:296`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L296) |
| `field` | `FixSessionMetrics` | `inbound_messages` | `: u64` | [`crates/of_fix/src/session.rs:298`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L298) |
| `field` | `FixSessionMetrics` | `outbound_messages` | `: u64` | [`crates/of_fix/src/session.rs:300`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L300) |
| `field` | `FixSessionMetrics` | `inbound_application_messages` | `: u64` | [`crates/of_fix/src/session.rs:302`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L302) |
| `field` | `FixSessionMetrics` | `outbound_application_messages` | `: u64` | [`crates/of_fix/src/session.rs:304`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L304) |
| `field` | `FixSessionMetrics` | `sequence_gaps` | `: u64` | [`crates/of_fix/src/session.rs:306`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L306) |
| `field` | `FixSessionMetrics` | `duplicate_messages` | `: u64` | [`crates/of_fix/src/session.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L308) |
| `field` | `FixSessionMetrics` | `sequence_too_low` | `: u64` | [`crates/of_fix/src/session.rs:310`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L310) |
| `field` | `FixSessionMetrics` | `resend_requests_sent` | `: u64` | [`crates/of_fix/src/session.rs:312`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L312) |
| `field` | `FixSessionMetrics` | `resend_requests_received` | `: u64` | [`crates/of_fix/src/session.rs:314`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L314) |
| `field` | `FixSessionMetrics` | `heartbeats_sent` | `: u64` | [`crates/of_fix/src/session.rs:316`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L316) |
| `field` | `FixSessionMetrics` | `heartbeats_received` | `: u64` | [`crates/of_fix/src/session.rs:318`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L318) |
| `field` | `FixSessionMetrics` | `test_requests_sent` | `: u64` | [`crates/of_fix/src/session.rs:320`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L320) |
| `field` | `FixSessionMetrics` | `test_requests_received` | `: u64` | [`crates/of_fix/src/session.rs:322`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L322) |
| `field` | `FixSessionMetrics` | `logons_sent` | `: u64` | [`crates/of_fix/src/session.rs:324`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L324) |
| `field` | `FixSessionMetrics` | `logons_received` | `: u64` | [`crates/of_fix/src/session.rs:326`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L326) |
| `field` | `FixSessionMetrics` | `logouts_sent` | `: u64` | [`crates/of_fix/src/session.rs:328`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L328) |
| `field` | `FixSessionMetrics` | `logouts_received` | `: u64` | [`crates/of_fix/src/session.rs:330`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L330) |
| `field` | `FixSessionMetrics` | `timeout_disconnects` | `: u64` | [`crates/of_fix/src/session.rs:332`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L332) |
| `field` | `FixSessionMetrics` | `last_inbound_ns` | `: Option<u64>` | [`crates/of_fix/src/session.rs:334`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L334) |
| `field` | `FixSessionMetrics` | `last_outbound_ns` | `: Option<u64>` | [`crates/of_fix/src/session.rs:336`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L336) |
| `variant` | `FixSessionError` | `MissingTag` | `MissingTag(FixTag)` | [`crates/of_fix/src/session.rs:358`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L358) |
| `variant` | `FixSessionError` | `MalformedNumericTag` | `MalformedNumericTag(FixTag)` | [`crates/of_fix/src/session.rs:360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L360) |
| `variant` | `FixSessionError` | `VersionMismatch` | `VersionMismatch` | [`crates/of_fix/src/session.rs:362`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L362) |
| `variant` | `FixSessionError` | `SenderCompIdMismatch` | `SenderCompIdMismatch` | [`crates/of_fix/src/session.rs:364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L364) |
| `variant` | `FixSessionError` | `TargetCompIdMismatch` | `TargetCompIdMismatch` | [`crates/of_fix/src/session.rs:366`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L366) |
| `variant` | `FixSessionError` | `UnexpectedResetSeqNumFlag` | `UnexpectedResetSeqNumFlag` | [`crates/of_fix/src/session.rs:375`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L375) |
| `variant` | `FixSessionError` | `TestRequestIdMismatch` | `TestRequestIdMismatch` | [`crates/of_fix/src/session.rs:391`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L391) |
| `variant` | `FixSessionError` | `UnexpectedApplicationMessage` | `UnexpectedApplicationMessage` | [`crates/of_fix/src/session.rs:393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L393) |
| `variant` | `FixSessionError` | `Encode` | `Encode(FixEncodeError)` | [`crates/of_fix/src/session.rs:395`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L395) |
| `variant` | `FixSessionError` | `Sequence` | `Sequence(FixSequenceError)` | [`crates/of_fix/src/session.rs:397`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_fix/src/session.rs#L397) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
