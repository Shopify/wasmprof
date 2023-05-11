use serde::Serialize;

#[no_mangle]
pub extern "C" fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {}

struct ReturnData {
    data: Vec<u8>,
}

#[no_mangle]
pub extern "C" fn __dummy_wasmprof_stacks_create() -> usize {
    let data = vec!["hello".to_string(), "world".to_string()];
    let mut buf = vec![];
    data.serialize(&mut rmp_serde::Serializer::new(&mut buf))
        .unwrap();
    let return_data = ReturnData { data: buf };
    let return_data = Box::new(return_data);
    Box::into_raw(return_data) as usize
}

#[no_mangle]
pub extern "C" fn __dummy_wasmprof_stacks_destroy(ptr: usize) {
    unsafe {
        let _ = Box::from_raw(ptr as *mut ReturnData);
    }
}

#[no_mangle]
pub extern "C" fn __dummy_wasmprof_stacks_get(ptr: usize) -> usize {
    let return_data = unsafe {
        let return_data: *const ReturnData = ptr as *const ReturnData;
        let return_data = (*return_data).data.as_ptr();
        return_data as usize
    };
    return_data
}

#[no_mangle]
pub extern "C" fn __dummy_wasmprof_stacks_len(ptr: usize) -> usize {
    let return_data = unsafe {
        let return_data: *const ReturnData = ptr as *const ReturnData;
        let return_data = (*return_data).data.len();
        return_data
    };
    return_data
}
