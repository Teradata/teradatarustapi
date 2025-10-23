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

	// Loop over args[3..] if available
	for i in (3..args.len()).step_by(2) {
		let request_text = &args[i];
		let bind_values = if i + 1 < args.len() {
			&args[i + 1]
		} else {
			"null"
		};

		execute_request(u_log, conn_handle, request_text, bind_values);
	}

	if let Err(err) = teradatarustapi::go_close_connection_wrapper(u_log, conn_handle) {
		println!("Error from go_close_connection_wrapper: {}", err);
	}
} // end main
