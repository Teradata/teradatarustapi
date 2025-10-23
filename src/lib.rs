// Copyright 2025 by Teradata Corporation. All Rights Reserved.

use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::env;
use std::ffi::{CString, CStr};
use std::fs;
use std::mem;
use std::os::raw::{c_char, c_ulonglong, c_ushort};
use std::path::PathBuf;
use std::ptr;
use std::sync::Arc;
use std::sync::OnceLock;
use libloading::{Library, Symbol};
use serde_json;

// Function pointer types matching the C function signatures

type GoCombineJSON = unsafe extern "C" fn(
	json1: *const c_char,
	json2: *const c_char,
	error: *mut *mut c_char,
	combined: *mut *mut c_char,
);

type GoParseParams = unsafe extern "C" fn(
	params: *const c_char,
	error: *mut *mut c_char,
	log: *mut c_ulonglong,
);

type GoCreateConnection = unsafe extern "C" fn(
	log: c_ulonglong,
	version: *const c_char,
	params: *const c_char,
	error: *mut *mut c_char,
	conn_handle: *mut c_ulonglong,
);

type GoCloseConnection = unsafe extern "C" fn(
	log: c_ulonglong,
	conn_handle: c_ulonglong,
	error: *mut *mut c_char,
);

type GoCancelRequest = unsafe extern "C" fn(
	log: c_ulonglong,
	conn_handle: c_ulonglong,
	error: *mut *mut c_char,
);

type RustGoCreateRows = unsafe extern "C" fn(
	log: c_ulonglong,
	conn_handle: c_ulonglong,
	request_text: *const c_char,
	bind_values: *const c_char,
	error: *mut *mut c_char,
	rows_handle: *mut c_ulonglong,
);

type RustGoResultMetaData = unsafe extern "C" fn(
	log: c_ulonglong,
	rows_handle: c_ulonglong,
	error: *mut *mut c_char,
	activity_count: *mut c_ulonglong,
	activity_type: *mut c_ushort,
	activity_name: *mut *mut c_char,
	column_metadata: *mut *mut c_char,
);

type RustGoFetchRow = unsafe extern "C" fn(
	log: c_ulonglong,
	rows_handle: c_ulonglong,
	error: *mut *mut c_char,
	column_values: *mut *mut c_char,
);

type GoNextResult = unsafe extern "C" fn(
	log: c_ulonglong,
	rows_handle: c_ulonglong,
	error: *mut *mut c_char,
	avail: *mut c_char,
);

type GoCloseRows = unsafe extern "C" fn(
	log: c_ulonglong,
	rows_handle: c_ulonglong,
	error: *mut *mut c_char,
);

type GoFreePointer = unsafe extern "C" fn(
	log: c_ulonglong,
	ptr: *mut c_char,
);

static GOSIDE_LIBRARY: OnceLock<Arc<Library>> = OnceLock::new();

static GO_COMBINE_JSON: OnceLock<Symbol<'static, GoCombineJSON>> = OnceLock::new();
static GO_PARSE_PARAMS: OnceLock<Symbol<'static, GoParseParams>> = OnceLock::new();
static GO_CREATE_CONNECTION: OnceLock<Symbol<'static, GoCreateConnection>> = OnceLock::new();
static GO_CLOSE_CONNECTION: OnceLock<Symbol<'static, GoCloseConnection>> = OnceLock::new();
static GO_CANCEL_REQUEST: OnceLock<Symbol<'static, GoCancelRequest>> = OnceLock::new();
static RUSTGO_CREATE_ROWS: OnceLock<Symbol<'static, RustGoCreateRows>> = OnceLock::new();
static RUSTGO_RESULT_METADATA: OnceLock<Symbol<'static, RustGoResultMetaData>> = OnceLock::new();
static RUSTGO_FETCH_ROW: OnceLock<Symbol<'static, RustGoFetchRow>> = OnceLock::new();
static GO_NEXT_RESULT: OnceLock<Symbol<'static, GoNextResult>> = OnceLock::new();
static GO_CLOSE_ROWS: OnceLock<Symbol<'static, GoCloseRows>> = OnceLock::new();
static GO_FREE_POINTER: OnceLock<Symbol<'static, GoFreePointer>> = OnceLock::new();

// Rust wrapper for goCombineJSON
fn go_combine_json_wrapper(
	json1: &str,
	json2: &str,
) -> Result<String, String> {
	let c_json1 = CString::new(json1).unwrap();
	let c_json2 = CString::new(json2).unwrap();
	let mut error: *mut c_char = ptr::null_mut();
	let mut combined: *mut c_char = ptr::null_mut();
	unsafe {
		GO_COMBINE_JSON.get().unwrap()(
			c_json1.as_ptr(),
			c_json2.as_ptr(),
			&mut error,
			&mut combined,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(0, error);
			return Err(err_str);
		}
		let result = CStr::from_ptr(combined).to_string_lossy().into_owned();
		go_free_pointer_wrapper(0, combined);
		Ok(result)
	}
}

// Rust wrapper for goParseParams
fn go_parse_params_wrapper(
	params: &str,
) -> Result<u64, String> {
	let c_params = CString::new(params).unwrap();
	let mut error: *mut c_char = ptr::null_mut();
	let mut u_log: u64 = 0;
	unsafe {
		GO_PARSE_PARAMS.get().unwrap()(
			c_params.as_ptr(),
			&mut error,
			&mut u_log,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(u_log)
	}
}

// Rust wrapper for goCreateConnection
fn go_create_connection_wrapper(
	u_log: u64,
	version: &str,
	params: &str,
) -> Result<u64, String> {
	let c_version = CString::new(version).unwrap();
	let c_params = CString::new(params).unwrap();
	let mut error: *mut c_char = ptr::null_mut();
	let mut conn_handle: u64 = 0;
	unsafe {
		GO_CREATE_CONNECTION.get().unwrap()(
			u_log,
			c_version.as_ptr(),
			c_params.as_ptr(),
			&mut error,
			&mut conn_handle,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(conn_handle)
	}
}

// Rust wrapper for goCloseConnection
pub fn go_close_connection_wrapper(
	u_log: u64,
	conn_handle: u64,
) -> Result<(), String> {
	let mut error: *mut c_char = ptr::null_mut();
	unsafe {
		GO_CLOSE_CONNECTION.get().unwrap()(u_log, conn_handle, &mut error);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(())
	}
}

// Rust wrapper for goCancelRequest
pub fn go_cancel_request_wrapper(
	u_log: u64,
	conn_handle: u64,
) -> Result<(), String> {
	let mut error: *mut c_char = ptr::null_mut();
	unsafe {
		GO_CANCEL_REQUEST.get().unwrap()(u_log, conn_handle, &mut error);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(())
	}
}

// Rust wrapper for rustgoCreateRows
pub fn rustgo_create_rows_wrapper(
	u_log: u64,
	conn_handle: u64,
	request_text: &str,
	bind_values: &str,
) -> Result<u64, String> {
	let c_request_text = CString::new(request_text).unwrap();
	let c_bind_values = CString::new(bind_values).unwrap();
	let mut error: *mut c_char = ptr::null_mut();
	let mut rows_handle: u64 = 0;
	unsafe {
		RUSTGO_CREATE_ROWS.get().unwrap()(
			u_log,
			conn_handle,
			c_request_text.as_ptr(),
			c_bind_values.as_ptr(),
			&mut error,
			&mut rows_handle,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(rows_handle)
	}
}

// Rust wrapper for rustgoResultMetaData
pub fn rustgo_result_metadata_wrapper(
	u_log: u64,
	rows_handle: u64,
) -> Result<(u64, u16, String, String), String> {
	let mut error: *mut c_char = ptr::null_mut();
	let mut activity_count: u64 = 0;
	let mut activity_type: u16 = 0;
	let mut activity_name: *mut c_char = ptr::null_mut();
	let mut column_metadata: *mut c_char = ptr::null_mut();
	unsafe {
		RUSTGO_RESULT_METADATA.get().unwrap()(
			u_log,
			rows_handle,
			&mut error,
			&mut activity_count,
			&mut activity_type,
			&mut activity_name,
			&mut column_metadata,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		let activity_name_str = CStr::from_ptr(activity_name).to_string_lossy().into_owned();
		let column_metadata_str = CStr::from_ptr(column_metadata).to_string_lossy().into_owned();
		go_free_pointer_wrapper(u_log, activity_name);
		go_free_pointer_wrapper(u_log, column_metadata);
		Ok((activity_count, activity_type, activity_name_str, column_metadata_str))
	}
}

// Rust wrapper for rustgoFetchRow
pub fn rustgo_fetch_row_wrapper(
	u_log: u64,
	rows_handle: u64,
) -> Result<Option<String>, String> {
	let mut error: *mut c_char = ptr::null_mut();
	let mut column_values: *mut c_char = ptr::null_mut();
	unsafe {
		RUSTGO_FETCH_ROW.get().unwrap()(
			u_log,
			rows_handle,
			&mut error,
			&mut column_values,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		if column_values.is_null() {
			// No more rows to fetch
			return Ok(None);
		}
		let column_values_str = CStr::from_ptr(column_values).to_string_lossy().into_owned();
		go_free_pointer_wrapper(u_log, column_values);
		Ok(Some(column_values_str))
	}
}

// Rust wrapper for goNextResult
pub fn go_next_result_wrapper(
	u_log: u64,
	rows_handle: u64,
) -> Result<bool, String> {
	let mut error: *mut c_char = ptr::null_mut();
	let mut avail: c_char = 0;
	unsafe {
		GO_NEXT_RESULT.get().unwrap()(
			u_log,
			rows_handle,
			&mut error,
			&mut avail,
		);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(avail == 'Y' as c_char)
	}
}

// Rust wrapper for goCloseRows
pub fn go_close_rows_wrapper(
	u_log: u64,
	rows_handle: u64,
) -> Result<(), String> {
	let mut error: *mut c_char = ptr::null_mut();
	unsafe {
		GO_CLOSE_ROWS.get().unwrap()(u_log, rows_handle, &mut error);
		if !error.is_null() {
			let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
			go_free_pointer_wrapper(u_log, error);
			return Err(err_str);
		}
		Ok(())
	}
}

// Rust wrapper for goFreePointer
fn go_free_pointer_wrapper(
	u_log: u64,
	ptr: *mut c_char
) {
	unsafe { GO_FREE_POINTER.get().unwrap()(u_log, ptr); }
}

fn get_extension() -> String {
	let os_type = env::consts::OS.to_lowercase();
	let cpu = env::consts::ARCH.to_lowercase();
	let b_arm = cpu.starts_with("arm") || cpu.starts_with("aarch");
	let b_power = cpu == "ppc64le";
	let b_fips = os_type == "linux" && fs::read_to_string("/proc/sys/crypto/fips_enabled").unwrap_or_default().trim() == "1";
	let n_bits = mem::size_of::<usize>() * 8;

	match os_type.as_str() {
		"windows" => {
			if n_bits == 32 {
				"x86.dll".to_string()
			} else {
				"dll".to_string()
			}
		}
		"macos" => "dylib".to_string(),
		"aix" => "aix.so".to_string(),
		_ => {
			if b_arm && b_fips {
				"arm.fips.so".to_string()
			} else if b_arm {
				"arm.so".to_string()
			} else if b_power {
				"power.so".to_string()
			} else if n_bits == 32 {
				"x86.so".to_string()
			} else if b_fips {
				"fips.so".to_string()
			} else {
				"so".to_string()
			}
		}
	}
} // end get_extension

pub fn load_driver(
	lib_dir: &str
) -> Result<(), String> {
	let extension = get_extension();

	let mut lib_path = PathBuf::from(lib_dir);
	lib_path.push(format!("teradatasql.{}", extension));

	// Only initialize the global library once
	match unsafe { Library::new(lib_path) } {
		Ok(lib) => {
			GOSIDE_LIBRARY.set(Arc::new(lib)).map_err(|_| "Library already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not load library: {}", err));
		}
	}

	let go_combine_json_result         = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoCombineJSON>        ("goCombineJSON"       .as_bytes()) };
	let go_parse_params_result         = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoParseParams>        ("goParseParams"       .as_bytes()) };
	let go_create_connection_result    = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoCreateConnection>   ("goCreateConnection"  .as_bytes()) };
	let go_close_connection_result     = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoCloseConnection>    ("goCloseConnection"   .as_bytes()) };
	let go_cancel_request_result       = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoCancelRequest>      ("goCancelRequest"     .as_bytes()) };
	let rustgo_create_rows_result      = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<RustGoCreateRows>     ("rustgoCreateRows"    .as_bytes()) };
	let rustgo_result_metadata_result  = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<RustGoResultMetaData> ("rustgoResultMetaData".as_bytes()) };
	let rustgo_fetch_row_result        = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<RustGoFetchRow>       ("rustgoFetchRow"      .as_bytes()) };
	let go_next_result_result          = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoNextResult>         ("goNextResult"        .as_bytes()) };
	let go_close_rows_result           = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoCloseRows>          ("goCloseRows"         .as_bytes()) };
	let go_free_pointer_result         = unsafe { GOSIDE_LIBRARY.get().unwrap().get::<GoFreePointer>        ("goFreePointer"       .as_bytes()) };

	match go_combine_json_result {
		Ok(f) => {
			GO_COMBINE_JSON.set(unsafe { mem::transmute::<Symbol<GoCombineJSON>, Symbol<'static, GoCombineJSON>>(f) }).map_err(|_| "goCombineJSON already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goCombineJSON: {}", err));
		}
	}

	match go_parse_params_result {
		Ok(f) => {
			GO_PARSE_PARAMS.set(unsafe { mem::transmute::<Symbol<GoParseParams>, Symbol<'static, GoParseParams>>(f) }).map_err(|_| "goParseParams already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goParseParams: {}", err));
		}
	}

	match go_create_connection_result {
		Ok(f) => {
			GO_CREATE_CONNECTION.set(unsafe { mem::transmute::<Symbol<GoCreateConnection>, Symbol<'static, GoCreateConnection>>(f) }).map_err(|_| "goCreateConnection already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goCreateConnection: {}", err));
		}
	}

	match go_close_connection_result {
		Ok(f) => {
			GO_CLOSE_CONNECTION.set(unsafe { mem::transmute::<Symbol<GoCloseConnection>, Symbol<'static, GoCloseConnection>>(f) }).map_err(|_| "goCloseConnection already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goCloseConnection: {}", err));
		}
	}

	match go_cancel_request_result {
		Ok(f) => {
			GO_CANCEL_REQUEST.set(unsafe { mem::transmute::<Symbol<GoCancelRequest>, Symbol<'static, GoCancelRequest>>(f) }).map_err(|_| "goCancelRequest already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goCancelRequest: {}", err));
		}
	}

	match rustgo_create_rows_result {
		Ok(f) => {
			RUSTGO_CREATE_ROWS.set(unsafe { mem::transmute::<Symbol<RustGoCreateRows>, Symbol<'static, RustGoCreateRows>>(f) }).map_err(|_| "rustgoCreateRows already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function rustgoCreateRows: {}", err));
		}
	}

	match rustgo_result_metadata_result {
		Ok(f) => {
			RUSTGO_RESULT_METADATA.set(unsafe { mem::transmute::<Symbol<RustGoResultMetaData>, Symbol<'static, RustGoResultMetaData>>(f) }).map_err(|_| "rustgoResultMetaData already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function rustgoResultMetaData: {}", err));
		}
	}

	match rustgo_fetch_row_result {
		Ok(f) => {
			RUSTGO_FETCH_ROW.set(unsafe { mem::transmute::<Symbol<RustGoFetchRow>, Symbol<'static, RustGoFetchRow>>(f) }).map_err(|_| "rustgoFetchRow already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function rustgoFetchRow: {}", err));
		}
	}

	match go_next_result_result {
		Ok(f) => {
			GO_NEXT_RESULT.set(unsafe { mem::transmute::<Symbol<GoNextResult>, Symbol<'static, GoNextResult>>(f) }).map_err(|_| "goNextResult already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goNextResult: {}", err));
		}
	}

	match go_close_rows_result {
		Ok(f) => {
			GO_CLOSE_ROWS.set(unsafe { mem::transmute::<Symbol<GoCloseRows>, Symbol<'static, GoCloseRows>>(f) }).map_err(|_| "goCloseRows already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goCloseRows: {}", err));
		}
	}

	match go_free_pointer_result {
		Ok(f) => {
			GO_FREE_POINTER.set(unsafe { mem::transmute::<Symbol<GoFreePointer>, Symbol<'static, GoFreePointer>>(f) }).map_err(|_| "goFreePointer already set".to_string())?;
		},
		Err(err) => {
			return Err(format!("Could not link to function goFreePointer: {}", err));
		}
	}

	Ok(())

} // end load_driver

pub fn create_connection(
	connect_params_json: &str,
) -> Result<(u64, u64), String> {

	// Backtrace::capture() captures a backtrace of the current OS thread according to the environment variable RUST_BACKTRACE
	// If RUST_BACKTRACE is not set, then Backtrace::capture() returns a disabled backtrace
	// Backtrace::force_capture() always forcibly captures a backtrace regardless of the RUST_BACKTRACE setting
	let stack_trace = Backtrace::force_capture();
	let stack_trace_str = format!("{}", stack_trace);

	let mut abbrev_stack_trace_str = String::new();
	for line in stack_trace_str.lines() {
		// Trim leading and trailing whitespace
		let trimmed_line = line.trim();
		// Use regular expression to trim leading number and colon if present
		let re = regex::Regex::new(r"^\d+:\s*").unwrap();
		let trimmed_line = re.replace(trimmed_line, "").to_string();

		// Replace all backslashes with forward slashes
		let trimmed_line = trimmed_line.replace("\\", "/");

		if trimmed_line == "std::rt::lang_start_internal" {
			break;
		}
		if trimmed_line.starts_with("std::") || trimmed_line.starts_with("core::") {
			continue;
		}

		// Trim "at " prefix if present
		let trimmed_line = if trimmed_line.starts_with("at ") {
			trimmed_line[3..].to_string()
		} else {
			trimmed_line
		};

		// Skip if trimmed_line contains /library/std/src/ or /library/core/src/
		if trimmed_line.contains("/library/std/src/") || trimmed_line.contains("/library/core/src/") {
			continue;
		}

		if !abbrev_stack_trace_str.is_empty() {
			abbrev_stack_trace_str.insert_str(0, " ");
		}

		abbrev_stack_trace_str.insert_str(0, &trimmed_line);
	}

	let mut map = HashMap::new();
	map.insert("client_kind", "U");
	map.insert("client_stack", &abbrev_stack_trace_str);
	let json_str = serde_json::to_string(&map).unwrap();

	let combined_json = match go_combine_json_wrapper(connect_params_json, json_str.as_str()) {
		Ok(combined_json) => combined_json,
		Err(err) => {
			return Err(format!("Error from go_combine_json_wrapper: {}", err));
		}
	};

	// Call go_parse_params_wrapper with the result from go_combine_json_wrapper
	let u_log = match go_parse_params_wrapper(combined_json.as_str()) {
		Ok(u_log) => u_log,
		Err(err) => {
			return Err(format!("Error from go_parse_params_wrapper: {}", err));
		}
	};

	let version_str = ""; // omit to use GoSQL Driver version
	let conn_handle = match go_create_connection_wrapper(u_log, version_str, combined_json.as_str()) {
		Ok(handle) => handle,
		Err(err) => {
			return Err(format!("Error from go_create_connection_wrapper: {}", err));
		}
	};

	Ok((u_log, conn_handle))

} // end create_connection

fn execute_simple_request(
	u_log: u64,
	conn_handle: u64,
	request_text: &str,
) -> Result<(), String> {

	let rows_handle = match rustgo_create_rows_wrapper(u_log, conn_handle, request_text, "null") { // JSON null for no bind values
		Ok(handle) => handle,
		Err(err) => {
			return Err(format!("Error from rustgo_create_rows_wrapper: {}", err));
		}
	};

	if let Err(err) = go_close_rows_wrapper(u_log, rows_handle) {
		return Err(format!("Error from go_close_rows_wrapper: {}", err));
	}

	Ok(())

} // end execute_simple_request

pub fn commit(
	u_log: u64,
	conn_handle: u64,
) -> Result<(), String> {

	execute_simple_request(u_log, conn_handle, "{fn teradata_commit}")

} // end commit

pub fn rollback(
	u_log: u64,
	conn_handle: u64,
) -> Result<(), String> {

	execute_simple_request(u_log, conn_handle, "{fn teradata_rollback}")

} // end rollback

pub fn set_autocommit(
	u_log: u64,
	conn_handle: u64,
	b: bool,
) -> Result<(), String> {

	execute_simple_request(u_log, conn_handle, &format!("{{fn teradata_nativesql}}{{fn teradata_autocommit_{}}}", if b { "on" } else { "off" }))

} // end set_autocommit
