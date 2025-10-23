// Copyright 2025 by Teradata Corporation. All Rights Reserved.

use std::env;

fn execute_request(u_log: u64, conn_handle: u64, request_text: &str, bind_values: &str) {

	println!();
	println!("request_text: {}", request_text);
	println!("bind_values:  {}", bind_values);

	let rows_handle = match teradatarustapi::rustgo_create_rows_wrapper(u_log, conn_handle, request_text, bind_values) {
		Ok(handle) => handle,
		Err(err) => {
			println!("Error from rustgo_create_rows_wrapper: {}", err);
			return;
		}
	};

	for result_num in 1.. {
		match teradatarustapi::rustgo_result_metadata_wrapper(u_log, rows_handle) {
			Ok((activity_count, activity_type, activity_name, column_metadata)) => {
				println!("Result {} activity_count:  {}", result_num, activity_count);
				println!("Result {} activity_type:   {}", result_num, activity_type);
				println!("Result {} activity_name:   {}", result_num, activity_name);
				println!("Result {} column_metadata: {}", result_num, column_metadata);
			}
			Err(err) => {
				println!("Error from rustgo_result_metadata_wrapper: {}", err);
				return;
			}
		}

		for row_num in 1.. {
			match teradatarustapi::rustgo_fetch_row_wrapper(u_log, rows_handle) {
				Ok(Some(row)) => {
					println!("Result {} row {}: {}", result_num, row_num, row);
				}
				Ok(None) => {
					// No more rows to fetch
					break;
				}
				Err(err) => {
					println!("Error from rustgo_fetch_row_wrapper: {}", err);
					break;
				}
			}
		} // end for row_num

		// Advance to next result
		match teradatarustapi::go_next_result_wrapper(u_log, rows_handle) {
			Ok(true) => { // another result available
				continue;
			}
			Ok(false) => { // no more results
				break;
			}
			Err(err) => {
				println!("Error from go_next_result_wrapper: {}", err);
				break;
			}
		}
	} // end for result_num

	if let Err(err) = teradatarustapi::go_close_rows_wrapper(u_log, rows_handle) {
		println!("Error from go_close_rows_wrapper: {}", err);
	}
} // end execute_request

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 3 {
		println!("Parameters: SharedLibraryDir ConnectParamsJSON");
		return;
	}

	let lib_dir = &args[1];
	let connect_params_json = &args[2];
	println!("lib_dir: {}", lib_dir);
	println!("connect_params_json: {}", connect_params_json);

	if let Err(err) = teradatarustapi::load_driver(lib_dir) {
		println!("Error from load_driver: {}", err);
		return;
	}

	let (u_log, conn_handle) = match teradatarustapi::create_connection(connect_params_json) {
		Ok((u_log, conn_handle)) => (u_log, conn_handle),
		Err(err) => {
			println!("Error from create_connection: {}", err);
			return;
		}
	};
	println!("conn_handle: {}", conn_handle);

	// show client attributes
	execute_request(u_log, conn_handle, "select * from DBC.SessionInfoV where SessionNo = session", "null"); // null means no bind values

	// show session attributes
	execute_request(u_log, conn_handle, "help session", "null"); // null means no bind values

	execute_request(u_log, conn_handle, "create volatile table vtab (c1 integer, c2 varchar(100)) on commit preserve rows", "null");
	// demonstrate single row of bind values
	execute_request(u_log, conn_handle, "insert into vtab values (?, ?)", r#"[[123,"hello"]]"#);
	// demonstrate two rows of bind values
	execute_request(u_log, conn_handle, "insert into vtab values (?, ?)", r#"[[456,"world"],[789,"foobar"]]"#);
	// demonstrate how to insert NULL bind value using JSON null
	execute_request(u_log, conn_handle, "insert into vtab values (?, ?)", "[[999,null]]");
	execute_request(u_log, conn_handle, "select * from vtab order by 1", "null");
	// Result 1 row 1: [123,"hello"]
	// Result 1 row 2: [456,"world"]
	// Result 1 row 3: [789,"foobar"]
	// Result 1 row 4: [999,null]
	execute_request(u_log, conn_handle, "drop table vtab", "null");

	// demonstrate multi-statement request
	execute_request(u_log, conn_handle, "select session ; select * from DBC.DBCInfo order by 1 ; select current_timestamp ; select * from DBC.DBCInfo order by 1 desc", "null");

	// demonstrate how result set BYTE and VARBYTE values are returned as base64 encoded strings
	execute_request(u_log, conn_handle, "select to_bytes('ABCD', 'ascii') as byte_val, from_bytes(byte_val, 'base64m') as display_byte_val_as_base64", "null");
	// Result 1 row 1: ["QUJDRA==","QUJDRA=="]

	// demonstrate how the to_bytes function must be used to create VARBYTE values from base64 encoded bind values
	execute_request(u_log, conn_handle, "select to_bytes(?, 'base64m') as bound_byte_val, from_bytes(bound_byte_val, 'ascii') as display_byte_val_as_varchar", r#"[["QUJDRA=="]]"#);
	// Result 1 row 1: ["QUJDRA==","ABCD"]

	// demonstrate all supported Teradata data types
	execute_request(u_log, conn_handle, r#"create volatile table vtab (
		c1 byteint,
		c2 smallint,
		c3 integer,
		c4 bigint,
		c5 float,
		c6 decimal(18, 3),
		c7 decimal(38, 5),
		c8 number,
		c9 char(3),
		c10 varchar(100),
		c11 byte(3),
		c12 varbyte(20),
		c13 date,
		c14 time,
		c15 time with time zone,
		c16 timestamp,
		c17 timestamp with time zone,
		c18 interval year(4),
		c19 interval year(4) to month,
		c20 interval month(4),
		c21 interval day(4),
		c22 interval day(4) to hour,
		c23 interval day(4) to minute,
		c24 interval day(4) to second,
		c25 interval hour(4),
		c26 interval hour(4) to minute,
		c27 interval hour(4) to second,
		c28 interval minute(4),
		c29 interval minute(4) to second,
		c30 interval second(4),
		c31 period(date),
		c32 period(time),
		c33 period(time with time zone),
		c34 period(timestamp),
		c35 period(timestamp with time zone),
		c36 blob,
		c37 clob,
		c38 xml,
		c39 json) on commit preserve rows"#, "null");
	execute_request(u_log, conn_handle,
		r#"insert into vtab (
			c1,  -- byteint
			c2,  -- smallint
			c3,  -- integer
			c4,  -- bigint
			c5,  -- float
			c6,  -- decimal(18, 3)
			c7,  -- decimal(38, 5)
			c8,  -- number
			c9,  -- char(3)
			c10, -- varchar(100)
			c11, -- byte(3)
			c12, -- varbyte(20)
			c13, -- date
			c14, -- time
			c15, -- time with time zone
			c16, -- timestamp
			c17, -- timestamp with time zone
			c18, -- interval year(4)
			c19, -- interval year(4) to month
			c20, -- interval month(4)
			c21, -- interval day(4)
			c22, -- interval day(4) to hour
			c23, -- interval day(4) to minute
			c24, -- interval day(4) to second
			c25, -- interval hour(4)
			c26, -- interval hour(4) to minute
			c27, -- interval hour(4) to second
			c28, -- interval minute(4)
			c29, -- interval minute(4) to second
			c30, -- interval second(4)
			c31, -- period(date)
			c32, -- period(time)
			c33, -- period(time with time zone)
			c34, -- period(timestamp)
			c35, -- period(timestamp with time zone)
			c36, -- blob
			c37, -- clob
			c38, -- xml
			c39  -- json
		)
		values (
			?, -- c1 byteint
			?, -- c2 smallint
			?, -- c3 integer
			?, -- c4 bigint
			?, -- c5 float
			?, -- c6 decimal(18, 3)
			?, -- c7 decimal(38, 5)
			?, -- c8 number
			?, -- c9 char(3)
			?, -- c10 varchar(100)
			to_bytes(?, 'base64m'), -- c11 byte(3)     - must use to_bytes for base64 encoded bind value
			to_bytes(?, 'base64m'), -- c12 varbyte(20) - must use to_bytes for base64 encoded bind value
			?, -- c13 date
			?, -- c14 time
			?, -- c15 time with time zone
			?, -- c16 timestamp
			?, -- c17 timestamp with time zone
			?, -- c18 interval year(4)
			?, -- c19 interval year(4) to month
			?, -- c20 interval month(4)
			?, -- c21 interval day(4)
			?, -- c22 interval day(4) to hour
			?, -- c23 interval day(4) to minute
			?, -- c24 interval day(4) to second
			?, -- c25 interval hour(4)
			?, -- c26 interval hour(4) to minute
			?, -- c27 interval hour(4) to second
			?, -- c28 interval minute(4)
			?, -- c29 interval minute(4) to second
			?, -- c30 interval second(4)
			?, -- c31 period(date)
			?, -- c32 period(time)
			?, -- c33 period(time with time zone)
			?, -- c34 period(timestamp)
			?, -- c35 period(timestamp with time zone)
			to_bytes(?, 'base64m'), -- c36 blob - must use to_bytes for base64 encoded bind value
			?, -- c37 clob
			createxml(?), -- c38 xml - must use createxml function to convert string bind value to XML
			? -- c39 json
		)"#,
		r#"[[
			127,
			32767,
			2147483647,
			"9223372036854775807",
			3.14159,
			"123456789012345.678",
			"12345678901234567890.12345",
			"12345678901234567890.123456789",
			"abc",
			"hello world",
			"eHl6",
			"QUE+QUE/QQ==",
			"2025-12-25",
			"11:22:33.123456",
			"11:22:33.123456+11:22",
			"2025-12-25 11:22:33.123456",
			"2025-12-25 11:22:33.123456+11:22",
			"-1234",
			"-1234-11",
			"-1234",
			"-1234",
			"-1234 11",
			"-1234 11:22",
			"-1234 11:22:33.123456",
			"-1234",
			"-1234:22",
			"-1234:22:33.123456",
			"-1234",
			"-1234:33.123456",
			"-1234.123456",
			"('2005-02-03', '2006-02-04')",
			"('11:22:33.123456', '11:22:33.123457')",
			"('11:22:33.123456+11:22', '11:22:33.123457+11:22')",
			"('2011-01-23 11:22:33.123456', '2011-01-23 11:22:33.123457')",
			"('2011-01-23 11:22:33.123456+11:22', '2011-01-23 11:22:33.123457+11:22')",
			"QUJDREVG",
			"ClobValue",
			"<foo>bar</foo>",
			"[1,2,3]"
		]]"#);
	execute_request(u_log, conn_handle,
		r#"select
			c1, -- byteint                             JSON number     127
			c2, -- smallint                            JSON number     32767
			c3, -- integer                             JSON number     2147483647
			c4, -- bigint                              JSON string     "9223372036854775807"
			c5, -- float                               JSON number     3.14159
			c6, -- decimal(18, 3)                      JSON string     "123456789012345.678"
			c7, -- decimal(38, 5)                      JSON string     "12345678901234567890.12345"
			c8, -- number                              JSON string     "12345678901234567890.123456789"
			c9, -- char(3)                             JSON string     "abc   "
			c10, -- varchar(100)                       JSON string     "hello world"
			c11, -- byte(3)                            base64 encoded  "eHl6"
			from_bytes(c11, 'ascii'), --                               "xyz" - also show ASCII representation of byte value
			c12, -- varbyte(20)                        JSON string     "QUE+QUE/QQ=="
			from_bytes(c12, 'ascii'), --                               "AA>AA?A" - also show ASCII representation of varbyte value
			c13, -- date                               JSON string     "2025-12-25"
			c14, -- time                               JSON string     "11:22:33.123456"
			c15, -- time with time zone                JSON string     "11:22:33.123456+11:22"
			c16, -- timestamp                          JSON string     "2025-12-25 11:22:33.123456"
			c17, -- timestamp with time zone           JSON string     "2025-12-25 11:22:33.123456+11:22"
			c18, -- interval year(4)                   JSON string     "-1234"
			c19, -- interval year(4) to month          JSON string     "-1234-11"
			c20, -- interval month(4)                  JSON string     "-1234"
			c21, -- interval day(4)                    JSON string     "-1234"
			c22, -- interval day(4) to hour            JSON string     "-1234 11"
			c23, -- interval day(4) to minute          JSON string     "-1234 11:22"
			c24, -- interval day(4) to second          JSON string     "-1234 11:22:33.123456"
			c25, -- interval hour(4)                   JSON string     "-1234"
			c26, -- interval hour(4) to minute         JSON string     "-1234:22"
			c27, -- interval hour(4) to second         JSON string     "-1234:22:33.123456"
			c28, -- interval minute(4)                 JSON string     "-1234"
			c29, -- interval minute(4) to second       JSON string     "-1234:33.123456"
			c30, -- interval second(4)                 JSON string     "-1234.123456"
			c31, -- period(date)                       JSON string     "2005-02-03,2006-02-04"
			c32, -- period(time)                       JSON string     "11:22:33.123456,11:22:33.123457"
			c33, -- period(time with time zone)        JSON string     "11:22:33.123456+11:22,11:22:33.123457+11:22"
			c34, -- period(timestamp)                  JSON string     "2011-01-23 11:22:33.123456,2011-01-23 11:22:33.123457"
			c35, -- period(timestamp with time zone)   JSON string     "2011-01-23 11:22:33.123456+11:22,2011-01-23 11:22:33.123457+11:22"
			c36, -- blob                               base64 encoded  "QUJDREVG"
			from_bytes(c36, 'ascii'), --                               "ABCDEF" - also show ASCII representation of blob value
			c37, -- clob                               JSON string     "ClobValue"
			c38, -- xml                                JSON string     "<foo>bar</foo>"
			c39 -- json                                JSON string     "[1,2,3]"
		from vtab
		order by 1"#, "null");

	if let Err(err) = teradatarustapi::go_close_connection_wrapper(u_log, conn_handle) {
		println!("Error from go_close_connection_wrapper: {}", err);
	}
} // end main
