use std::ptr::null_mut;

use crate::graph::Graph;
use crate::plan::Plan;
use std::time::Instant;
<<<<<<< Updated upstream

=======
use std::ptr;
>>>>>>> Stashed changes

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
    pub mat: *mut libc::c_void,
    pub res_mat: *mut libc::c_void,
}

#[link(name = "lagraphx")]
extern "C" {
    pub fn LAGraph_DestroyRpqMatrixPlan(plan: *mut RpqMatrixPlan);
<<<<<<< Updated upstream
    pub fn LAGraph_RpqMatrix_getnnz(plan: *mut RpqMatrixPlan) -> libc::c_longlong;
    pub fn LAGraph_RPQMatrix(
=======
    pub fn LAGraph_RPQMatrix(
        ans: *mut usize,
>>>>>>> Stashed changes
        plan: *mut RpqMatrixPlan,
        msg: *mut libc::c_char,
    ) -> libc::c_longlong;
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

pub fn eval(graph: &Graph, expr: egg::RecExpr<Plan>) -> Result<usize, String> {
    let mut plans: Vec<RpqMatrixPlan> = vec![
        RpqMatrixPlan {
            op: RpqMatrixOp::Label,
            lhs: null_mut(),
            rhs: null_mut(),
            mat: null_mut(),
            res_mat: null_mut()
        };
        expr.len()
    ];
    expr.items().for_each(|(id, plan)| {
        let eval_plan = match plan {
            &Plan::Seq([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Concat,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::Alt([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Lor,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::Star([lhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::Kleene,
                lhs: null_mut(),
                rhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::LStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneL,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
                res_mat: null_mut(),
                mat: null_mut(),
            },
            &Plan::RStar([lhs, rhs]) => RpqMatrixPlan {
                op: RpqMatrixOp::KleeneR,
                lhs: plans.get_mut::<usize>(lhs.into()).unwrap() as *mut RpqMatrixPlan,
                rhs: plans.get_mut::<usize>(rhs.into()).unwrap() as *mut RpqMatrixPlan,
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
        plans[std::convert::Into::<usize>::into(id)] = eval_plan;
    });
    let plan = plans.iter_mut().last().unwrap();
    
    unsafe {
        let start = Instant::now();
<<<<<<< Updated upstream
        let res = LAGraph_RPQMatrix(plan as *mut RpqMatrixPlan, null_mut());
        let elapsed = start.elapsed();
        println!("ns:{}",elapsed.as_nanos());
        assert_eq!(res, 0);
    };

    unsafe{
        let nnz = LAGraph_RpqMatrix_getnnz(plan as *mut RpqMatrixPlan);
        println!("nnz: {}",nnz);
        println!("");
        Ok(0)
=======
        let mut err_ptr: *mut std::os::raw::c_char = ptr::null_mut();
        let res  = LAGraph_RPQMatrix(&mut ans, plan as *mut RpqMatrixPlan, err_ptr);
        // println!("{}", res);
        assert_eq!(res,0);
        let elapsed = start.elapsed();    
        println!("{};{}", ans,elapsed.as_nanos());
        // LAGraph_DestroyRpqMatrixPlan(plan);
>>>>>>> Stashed changes
    }
}