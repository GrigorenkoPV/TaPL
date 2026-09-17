use core::mem;

use crate::{
    ast::{
        Address, Annotation, BinOp, Binding, Catch, Decl, Expr, Function, HasVisitable, Integer,
        Pattern, PatternBinding, Program, Type, Visitor,
    },
    extensions::{Extension, Extensions},
};

struct GateCollector {
    required: Extensions,
    is_in_global_scope: bool,
}

impl GateCollector {
    #[must_use]
    const fn new() -> Self {
        Self {
            required: Extensions::new(),
            is_in_global_scope: true,
        }
    }
    fn require(&mut self, extension: Extension) {
        let _: bool = self.required.insert(extension);
    }
}

impl<'ctx> Visitor<'ctx> for GateCollector {
    fn visit_function(&mut self, function: &Function<'ctx>) {
        let Function { params, .. } = function;

        match params.len() {
            0 => self.require(Extension::NullaryFunctions),
            1 => {}
            _ => self.require(Extension::MultiparameterFunctions),
        }
        // TODO: more checks

        let was_in_global_scope = mem::replace(&mut self.is_in_global_scope, false);
        if !was_in_global_scope {
            self.require(Extension::NestedFunctionDeclarations)
        }
        function.visit_inner_with(self);
        self.is_in_global_scope = was_in_global_scope;
    }

    fn visit_int(&mut self, int: &Integer) {
        if *int != 0 {
            todo!()
        }
    }

    fn visit_annotation(&mut self, _: &Annotation) {
        todo!()
    }

    fn visit_bin_op(&mut self, _: &BinOp) {
        todo!()
    }

    fn visit_extension(&mut self, _: &Extension) {
        todo!()
    }

    fn visit_addr(&mut self, _: &Address<'ctx>) {
        todo!()
    }

    fn visit_pattern_binding(&mut self, pattern_binding: &PatternBinding<'ctx>) {
        todo!();
        pattern_binding.visit_inner_with(self)
    }

    fn visit_binding(&mut self, binding: &Binding<'ctx>) {
        todo!();
        binding.visit_inner_with(self)
    }

    fn visit_decl(&mut self, decl: &Decl<'ctx>) {
        todo!();
        decl.visit_inner_with(self)
    }

    fn visit_ty(&mut self, ty: &Type<'ctx>) {
        todo!();
        ty.visit_inner_with(self)
    }

    fn visit_pat(&mut self, pat: &Pattern<'ctx>) {
        todo!();
        pat.visit_inner_with(self)
    }

    fn visit_catch(&mut self, catch: &Catch<'ctx>) {
        todo!();
        catch.visit_inner_with(self)
    }

    fn visit_expr(&mut self, expr: &Expr<'ctx>) {
        todo!();
        expr.visit_inner_with(self)
    }

    fn visit_program(&mut self, program: &Program<'ctx>) {
        todo!();
        program.visit_inner_with(self)
    }
}

pub(super) fn missing_extensions(program: &Program<'_>) -> Extensions {
    let mut collector = GateCollector::new();
    collector.visit_program(program);
    let GateCollector {
        mut required,
        is_in_global_scope: _,
    } = collector;
    for extension in &program.extensions {
        let _: bool = required.remove(extension);
    }
    required
}
