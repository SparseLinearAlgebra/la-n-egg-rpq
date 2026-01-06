# LA'n'EGG RPQ

**Linear Algebra &amp; E-Graphs for Regular Path Queries** is a project focusing on applying [equality saturation via E-graphs](https://egraphs-good.github.io/) to optimize [regular path queries](https://en.wikipedia.org/wiki/Regular_path_query) evaluation expressed in terms of [sparse linear algebra](https://graphblas.org/).

## Dependencies

This project uses the following dependencies.

* Rust compiler (2021 edition).
* [LAGraph](https://github.com/GraphBLAS/LAGraph) and [GraphBLAS](https://github.com/DrTimothyAldenDavis/GraphBLAS) libraries (packed as submodules in `vendor/`).

## Building

Start from installing [LAGraph](https://github.com/GraphBLAS/LAGraph) using the [QUICK START instructions from its repository](https://github.com/DrTimothyAldenDavis/GraphBLAS/blob/stable/README.md).

You might not install [LAGraph](https://github.com/GraphBLAS/LAGraph) on a system level and simply use local build as a submodule by running.

```bash
git submodule update --init --recursive
```

To build the project itself after installing the dependencies, clone the repository and execute the following commands.

```bash
cargo build --release
```

The binary will be available in the `target/release/` directory

## Usage

Basically, the binary can be used as follows.

```bash
la-n-egg-rpq <path-to-graph-as-matrix-market-files> <queries-file>
```

To convert the graph, please, use [our tools](https://github.com/SparseLinearAlgebra/la-rpq) for converting them into [the MatrixMarket format](https://math.nist.gov/MatrixMarket/formats.html). You might start from [one of the prepared datasets](https://github.com/SparseLinearAlgebra/la-rpq/tree/main/Datasets).

The queries should be expressed in the SPARQL format. They should use `<query-number>,<Simplified SPARQL format>`. Here is an example.

```
1,<Radosha_Ferioli> (<coauthor>)* ?obj
2,<Mikolas_Sirman> (<coauthor>)* ?obj
3,<Bruna_Nervis> (<coauthor>)* ?obj
4,<Amishi_Masgalas> (<coauthor>)* ?obj
5,<Socorro_Mcsorley> (<coauthor>)* ?obj
```

By default the binary executes randomly generated query plans for each query from the plan and printing best/worst/mean time for each query kind.

## Authors

* [Georgiy Belyanin](https://github.com/georgiy-belyanin) (mail: [belyaningeorge@ya.ru](mailto://belyaningeorge@ya.ru)).
* [Rodion Suvorov](https://github.com/suvorovrain) (mail: [rodion.suvorov.94@mail.ru](mailto://rodion.suvorov.94@mail.ru)).
* [Semyon Grigorev](https://github.com/gsvgit) (mail: [s.v.grigoriev@mail.spbu.ru](mailto://s.v.grigoriev@mail.spbu.ru)).

## Acknowledgments & copyright notices

LA'n'EGG RPQ is licensed under the [Apache 2.0 License](https://www.apache.org/licenses/LICENSE-2.0).

Thank you for you interest in the la-n-egg-rpq project.
