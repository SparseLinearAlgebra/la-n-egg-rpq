use std::ptr::null_mut;

use crate::graph::Graph;
use crate::grb;
use crate::plan::Plan;

#[repr(C)]
#[derive(Clone)]
pub enum RpqMatrixOp {
    Label,
    Lor,
    Concat,
    Kleene,
    KleeneL,
    KleeneR,
}

#[repr(C)]
#[derive(Clone)]
pub struct RpqMatrixPlan {
    pub op: RpqMatrixOp,
    pub lhs: *mut RpqMatrixPlan,
    pub rhs: *mut RpqMatrixPlan,
    pub mat: grb::Matrix,
    pub res_mat: grb::Matrix,
}

#[link(name = "lagraphx")]
extern "C" {
    pub fn LAGraph_Init(msg: *mut libc::c_char) -> libc::c_int;
    pub fn LAGraph_DestroyRpqMatrixPlan(plan: *mut RpqMatrixPlan);
    pub fn LAGraph_RPQMatrix(
        ans: *mut usize,
        plan: *mut RpqMatrixPlan,
        msg: *mut libc::c_char,
    ) -> libc::c_longlong;
    pub fn LAGraph_RPQMatrix_label(
        mat: *mut grb::Matrix,
        x: usize,
        i: usize,
        j: usize,
    ) -> libc::c_longlong;
}

#[link(name = "lagraph")]
extern "C" {
    pub fn LAGraph_MMRead(
        mat: *mut grb::Matrix,
        f: *mut libc::FILE,
        msg: *mut libc::c_char,
    ) -> libc::c_int;
}

pub fn eval(graph: &Graph, expr: egg::RecExpr<Plan>) -> Result<usize, String> {
    let mut plans: Vec<RpqMatrixPlan> = vec![
        RpqMatrixPlan {
            op: RpqMatrixOp::Label,
            lhs: null_mut(),
            rhs: null_mut(),
            mat: grb::Matrix::null(),
            res_mat: grb::Matrix::null()
        };
        expr.len()
    ];
    expr.items().for_each(|(id, plan)| {
        let eval_plan = match plan {
            &Plan::Seq([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Concat,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: grb::Matrix::null(),
                mat: grb::Matrix::null(),
            },
            &Plan::Alt([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Lor,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: grb::Matrix::null(),
                mat: grb::Matrix::null(),
            },
            &Plan::Star([lhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Kleene,
                lhs: null_mut(),
                rhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: grb::Matrix::null(),
                mat: grb::Matrix::null(),
            },
            &Plan::LStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneL,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: grb::Matrix::null(),
                mat: grb::Matrix::null(),
            },
            &Plan::RStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneR,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: grb::Matrix::null(),
                mat: grb::Matrix::null(),
            },
            Plan::Label(meta) => {
                let mut mat: grb::Matrix = grb::Matrix(std::ptr::null_mut());
                let mat = graph
                    .mats
                    .get(&meta.name)
                    .or({
                        graph.verts.get(&meta.name).map(|vert_idx| {
                            unsafe {
                                LAGraph_RPQMatrix_label(
                                    &mut mat as *mut grb::Matrix,
                                    *vert_idx - 1,
                                    graph.verts.len(),
                                    graph.verts.len(),
                                );
                            }
                            &mat
                        })
                    })
                    .unwrap()
                    .clone();
                RpqMatrixPlan {
                    op: RpqMatrixOp::Label,
                    lhs: null_mut(),
                    rhs: null_mut(),
                    res_mat: grb::Matrix::null(),
                    mat,
                }
            }
        };
        plans[std::convert::Into::<usize>::into(id)] = eval_plan;
    });
    let plan = plans.iter_mut().last().unwrap();
    let mut ans: usize = 0;
    unsafe {
        LAGraph_RPQMatrix(&mut ans, plan as *mut RpqMatrixPlan, null_mut());
        LAGraph_DestroyRpqMatrixPlan(plan);
    }
    Ok(ans)
}
