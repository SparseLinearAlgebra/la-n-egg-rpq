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
    pub fn LAGraph_DestroyRpqMatrixPlan(plan: *mut RpqMatrixPlan) -> libc::c_longlong;
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
    pub fn LAGraph_RPQMatrix_reduce(
        res: *mut usize,
        mat: grb::Matrix,
        reduce_type: u8, // reduce_type values:
                         // 0 --- reduce by row
                         // 1 --- reduce by col
    ) -> libc::c_int;
    pub fn LAGraph_RPQMatrix_Free(res: *mut grb::Matrix) -> libc::c_longlong;

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
            lhs: std::ptr::null_mut(),
            rhs: std::ptr::null_mut(),
            mat: grb::Matrix::null(),
            res_mat: grb::Matrix::null(),
        };
        expr.len()
    ];

    let mut owns_label_mat = vec![false; expr.len()];

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
                let mat = if let Some(m) = graph.mats.get(&meta.name) {
                    m.clone()
                } else {
                    let vert_idx = graph.verts.get(&meta.name).unwrap();
                    let mut tmp = grb::Matrix::null();
                    unsafe {
                        LAGraph_RPQMatrix_label(
                            &mut tmp as *mut grb::Matrix,
                            *vert_idx - 1,
                             graph.size,
                             graph.size,
                        )
                    };
                    owns_label_mat[std::convert::Into::<usize>::into(id)] = true;
                    tmp
                };
                RpqMatrixPlan {
                    op: RpqMatrixOp::Label,
                    lhs: std::ptr::null_mut(),
                    rhs: std::ptr::null_mut(),
                    mat,
                    res_mat: grb::Matrix::null(),
                }
            }
        };
        plans[std::convert::Into::<usize>::into(id)] = eval_plan;
    });

    let plan = plans.iter_mut().last().unwrap();

    let mut ans: usize = 0;
    let info =
        unsafe { LAGraph_RPQMatrix(&mut ans, plan as *mut RpqMatrixPlan, std::ptr::null_mut()) };
    if info != 0 {
        return Err(format!("solver failed: {}", info));
    }

    let info = unsafe { LAGraph_DestroyRpqMatrixPlan(plan as *mut RpqMatrixPlan) };
    if info != 0 {
        return Err(format!("destroy plan failed: {}", info));
    }

    plans.iter_mut().enumerate().for_each(|(i, p)| {
        if owns_label_mat[i] {
            unsafe {
                LAGraph_RPQMatrix_Free(&mut p.mat as *mut grb::Matrix);
            }
            p.mat = grb::Matrix::null();
            p.res_mat = grb::Matrix::null();
        }
    });
    Ok(ans)
}
