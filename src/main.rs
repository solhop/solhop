use rsat::cdcl;
use rsat::cdcl::DratClause;
use solhop_types::dimacs::{parse_dimacs_from_file, Dimacs};
use solhop_types::{Solution, Var};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name="solhop", about=env!("CARGO_PKG_DESCRIPTION"), version=env!("CARGO_PKG_VERSION"),
author=env!("CARGO_PKG_AUTHORS"), setting=structopt::clap::AppSettings::ColoredHelp)]
enum Opt {
    /// SAT Solver
    #[structopt(setting=structopt::clap::AppSettings::ColoredHelp)]
    Rsat {
        /// Input file in DIMACS format
        #[structopt(parse(from_os_str))]
        file: PathBuf,
        /// Algorithm to use (1 -> CDCL, 2 -> SLS)
        #[structopt(short, long, default_value = "1")]
        alg: u32,
        /// Enables data parallelism (currently only for sls solver)
        #[structopt(short, long)]
        parallel: bool,
        /// Maximum number of tries for SLS
        #[structopt(long = "max-tries", default_value = "100")]
        max_tries: u32,
        /// Maxinum number of flips in each try of SLS
        #[structopt(long = "max-flips", default_value = "1000")]
        max_flips: u32,
        /// Drat file to log conflict clauses addition and deletion
        #[structopt(long, parse(from_os_str))]
        drat: Option<PathBuf>,
    },
    /// MaxSAT Solver (Not yet implemented)
    #[structopt(setting=structopt::clap::AppSettings::ColoredHelp)]
    Msat {},
}

// Function to write drat clauses to file
pub fn write_drat_clauses(drat_file: &mut File, solver: rsat::cdcl::Solver) {
    if let Some(drat_clauses) = solver.drat_clauses() {
        for drat_clause in drat_clauses {
            let (is_delete, lits) = match drat_clause {
                DratClause::Add(lits) => (false, lits),
                DratClause::Delete(lits) => (true, lits),
            };
            if is_delete {
                write!(drat_file, "d ").unwrap();
            }
            for lit in lits.iter() {
                write!(
                    drat_file,
                    "{} ",
                    if lit.sign() {
                        -(lit.var().index() as i32 + 1)
                    } else {
                        lit.var().index() as i32 + 1
                    }
                )
                .unwrap();
            }
            writeln!(drat_file, "0").unwrap();
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    match opt {
        Opt::Rsat {
            alg,
            file,
            parallel,
            drat,
            max_tries,
            max_flips,
            ..
        } => {
            let (n_vars, clauses) =
                if let Dimacs::Cnf { n_vars, clauses } = parse_dimacs_from_file(&file) {
                    (n_vars, clauses)
                } else {
                    panic!("Incorrect input format");
                };

            let solution = match alg {
                1 => {
                    if parallel {
                        panic!("Parallelism is not implemented for CDCL solver yet.");
                    }

                    use cdcl::{Solver, SolverOptions};

                    let mut options = SolverOptions::default();
                    // options.branching_heuristic = BranchingHeuristic::Vsids {
                    //     var_inc: 1.0,
                    //     var_decay: 0.95,
                    // };
                    let mut drat = match drat {
                        Some(drat) => Some(File::create(drat).expect("Drat file not found")),
                        None => None,
                    };
                    if drat.is_some() {
                        options.capture_drat = true;
                    }
                    let mut solver = Solver::new(options);

                    let _vars: Vec<Var> = (0..n_vars).map(|_| solver.new_var()).collect();

                    for clause in clauses {
                        solver.add_clause(clause);
                    }

                    let solution = solver.solve(vec![]);

                    if let Solution::Unsat = solution {
                        if let Some(drat_file) = &mut drat {
                            write_drat_clauses(drat_file, solver);
                        }
                    }
                    solution
                }
                2 => {
                    let mut solver = rsat::sls::Solver::new_from_file(file.to_str().unwrap());
                    solver.local_search(max_tries, max_flips, rsat::sls::ScoreFnType::Exp, parallel)
                }
                _ => panic!("Invalid algorithm"),
            };
            match solution {
                Solution::Unsat => println!("s UNSATISFIABLE"),
                Solution::Unknown => println!("s UNKNOWN"),
                Solution::Best(solution) => {
                    println!("s UNKNOWN");
                    let solution = solution.iter().map(|&x| if x { 1 } else { -1 });
                    print!("v ");
                    for (i, v) in solution.enumerate() {
                        print!("{} ", v * ((i + 1) as i32));
                    }
                    println!("0");
                }
                Solution::Sat(solution) => {
                    println!("s SATISFIABLE");
                    print!("v ");
                    let solution = solution.iter().map(|&x| if x { 1 } else { -1 });
                    for (i, v) in solution.enumerate() {
                        print!("{} ", v * ((i + 1) as i32));
                    }
                    println!("0");
                }
            }
        }
        Opt::Msat { .. } => {
            todo!("msat");
        }
    }
}
