//! Integration harness for keyword fusion `match_util` unit tests.

mod match_util_module {
    pub(crate) use aho_corasick::PatternID;
    pub(crate) use lance::deps::arrow_array::StringArray;
    pub(crate) use xiuxian_vector::keyword::fusion::{
        build_name_lower_arrow, build_name_token_automaton_with_phrase,
        count_name_token_matches_and_exact, lowercase_string_array,
    };

    mod tests;
}
