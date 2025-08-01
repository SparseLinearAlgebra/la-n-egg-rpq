use std::ptr::null_mut;

use crate::graph::Graph;
use crate::plan::Plan;
use std::collections::HashMap;

#[repr(C)]
pub enum RpqMatrixOp {
    Label,
    Lor,
    Concat,
    Kleene,
    KleeneL,
    KleeneR,
}

#[repr(C)]
pub struct RpqMatrixPlan {
    pub op: RpqMatrixOp,
    pub lhs: *mut RpqMatrixPlan,
    pub rhs: *mut RpqMatrixPlan,
    pub mat: *mut libc::c_void,
    pub res_mat: *mut libc::c_void,
}

#[link(name = "lagraphx")]
extern "C" {
    pub fn LAGraph_RpqMatrix_initialize() -> libc::c_longlong;
    pub fn LAGraph_RPQMatrix(plan: *mut RpqMatrixPlan, msg: *mut libc::c_char) -> libc::c_longlong;
    pub fn LAGraph_RPQMatrix_label(
        mat: *mut *mut libc::c_void,
        x: usize,
        i: usize,
        j: usize,
    ) -> libc::c_longlong;
}

#[link(name = "lagraph")]
extern "C" {
    pub fn LAGraph_MMRead(
        mat: *mut *mut libc::c_void,
        f: *mut libc::FILE,
        msg: *mut libc::c_char,
    ) -> libc::c_int;
}

pub fn eval(graph: &Graph, expr: egg::RecExpr<Plan>) {
    let mut plans: HashMap<egg::Id, RpqMatrixPlan> = HashMap::with_capacity(expr.len());
    expr.items().for_each(|(id, plan)| {
        let eval_plan = match plan {
            &Plan::Seq([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Concat,
                lhs: plans.get_mut(&lhs).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut(&rhs).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::Alt([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Lor,
                lhs: plans.get_mut(&lhs).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut(&rhs).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::Star([lhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Kleene,
                lhs: null_mut(),
                rhs: plans.get_mut(&lhs).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::LStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneL,
                lhs: plans.get_mut(&lhs).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut(&rhs).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::RStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneR,
                lhs: plans.get_mut(&lhs).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut(&rhs).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            Plan::Label(meta) => {
                let mut mat: *mut libc::c_void = std::ptr::null_mut();
                let mat = *graph
                    .mats
                    .get(&meta.name)
                    .or({
                        graph.verts.get(&meta.name).map(|vert_idx| {
                            unsafe {
                                LAGraph_RPQMatrix_label(
                                    &mut mat as *mut *mut libc::c_void,
                                    *vert_idx,
                                    graph.verts.len(),
                                    graph.verts.len(),
                                );
                            }
                            &mat
                        })
                    })
                    .unwrap();
                RpqMatrixPlan {
                    op: RpqMatrixOp::Label,
                    lhs: null_mut(),
                    rhs: null_mut(),
                    res_mat: null_mut(),
                    mat,
                }
            }
        };
        plans.insert(id, eval_plan);
    });
    let (_id, plan) = plans.iter_mut().last().unwrap();
    unsafe {
        LAGraph_RPQMatrix(plan as *mut RpqMatrixPlan, null_mut());
    }
}
