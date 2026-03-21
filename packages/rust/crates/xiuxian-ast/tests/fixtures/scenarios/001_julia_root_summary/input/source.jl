module SamplePkg

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
