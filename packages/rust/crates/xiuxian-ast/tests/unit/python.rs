//! Package-top harness for Python helper unit tests.

use xiuxian_ast::{
    extract_docstring_from_match, extract_python_docstring, find_python_async_functions,
    find_python_classes, find_python_functions,
};

#[path = "python_tests.rs"]
mod python_tests;
