use std::error::Error;

use insta::assert_debug_snapshot;

use super::TreeSitterJuliaParser;

#[test]
fn parse_summary_extracts_root_julia_concepts() -> Result<(), Box<dyn Error>> {
    let mut parser = TreeSitterJuliaParser::new()?;
    let summary = parser.parse_summary(
        r#"module SamplePkg

export solve, Problem
using LinearAlgebra
@reexport using SciMLBase

"""
Problem docs.
"""
struct Problem
    x::Int
end

"""
Solve docs.
"""
function solve(problem::Problem)
    problem.x
end

"""
fastsolve docs.
"""
fastsolve(problem::Problem) = problem.x

end
"#,
    )?;

    assert_debug_snapshot!("julia_root_summary", summary);
    Ok(())
}

#[test]
fn parse_file_summary_extracts_includes_without_module() -> Result<(), Box<dyn Error>> {
    let mut parser = TreeSitterJuliaParser::new()?;
    let summary = parser.parse_file_summary(
        r#"
"""
Fast solve docs.
"""
fastsolve(problem::Problem) = problem.x

include("nested/extra.jl")
"#,
    )?;

    assert_debug_snapshot!("julia_file_summary", summary);
    Ok(())
}
