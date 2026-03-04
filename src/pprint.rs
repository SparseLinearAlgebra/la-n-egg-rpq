use crate::plan::Plan;
use egg::{Id, RecExpr};

pub fn pretty(expr: &RecExpr<Plan>) -> String {
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    enum Prec {
        Alt = 0,
        Seq = 1,
        Star = 2,
        Atom = 3,
    }

    fn print(expr: &RecExpr<Plan>, id: Id) -> (String, Prec) {
        match &expr[id] {
            Plan::Label(meta) => (format!("<{}#{}>", meta.name, meta.nvals), Prec::Atom),

            Plan::Alt([a, b]) => {
                let (sa, pa) = print(expr, *a);
                let (sb, pb) = print(expr, *b);

                let left = if pa < Prec::Alt {
                    format!("({})", sa)
                } else {
                    sa
                };
                let right = if pb < Prec::Alt {
                    format!("({})", sb)
                } else {
                    sb
                };

                (format!("{}|{}", left, right), Prec::Alt)
            }

            Plan::Seq([a, b]) => {
                let (sa, pa) = print(expr, *a);
                let (sb, pb) = print(expr, *b);

                let left = if pa <= Prec::Seq {
                    format!("({})", sa)
                } else {
                    sa
                };

                let right = if pb <= Prec::Seq {
                    format!("({})", sb)
                } else {
                    sb
                };

                (format!("{}/{}", left, right), Prec::Seq)
            }

            Plan::Star([a]) => {
                let (sa, pa) = print(expr, *a);

                let inner = if pa < Prec::Star {
                    format!("({})", sa)
                } else {
                    sa
                };

                (format!("{}*", inner), Prec::Star)
            }

            Plan::LStar([a, b]) => {
                let (sa, pa) = print(expr, *a);
                let (sb, pb) = print(expr, *b);

                let left = if pa < Prec::Star {
                    format!("({})", sa)
                } else {
                    sa
                };

                let right = if pb < Prec::Star {
                    format!("({})", sb)
                } else {
                    sb
                };

                (format!("({},{})l*", left, right), Prec::Star)
            }

            Plan::RStar([a, b]) => {
                let (sa, pa) = print(expr, *a);
                let (sb, pb) = print(expr, *b);

                let left = if pa < Prec::Star {
                    format!("({})", sa)
                } else {
                    sa
                };

                let right = if pb < Prec::Star {
                    format!("({})", sb)
                } else {
                    sb
                };

                (format!("({},{})*r", left, right), Prec::Star)
            }
        }
    }

    let root = Id::from(expr.as_ref().len() - 1);
    print(expr, root).0
}
