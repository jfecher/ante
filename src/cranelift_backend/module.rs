use std::path::Path;

use cranelift::prelude::{
    isa::{self, TargetFrontendConfig},
    settings, Configurable,
};
use cranelift_jit::JITBuilder;
use cranelift_module::{DataContext, FuncId, Linkage, Module};
use cranelift_object::ObjectBuilder;

use crate::util;

#[allow(clippy::large_enum_variant)]
pub enum DynModule {
    Jit(cranelift_jit::JITModule),
    Static(cranelift_object::ObjectModule),
}

impl DynModule {
    pub fn new(output_name: String, use_jit: bool) -> (Self, TargetFrontendConfig) {
        let mut settings = settings::builder();

        // Cranelift-jit currently only supports PIC on x86-64 and
        // will panic by default if it is enabled on other platforms
        if cfg!(not(target_arch = "x86_64")) {
            settings.set("use_colocated_libcalls", "false").unwrap();
            settings.set("is_pic", "false").unwrap();
        }

        let shared_flags = settings::Flags::new(settings);

        // TODO: Should we use cranelift_native here to get the native target instead?
        let target_isa = isa::lookup(target_lexicon::Triple::host())
            .unwrap()
            .finish(shared_flags)
            .unwrap();

        let frontend_config = target_isa.frontend_config();

        let libcall_names = cranelift_module::default_libcall_names();

        let module = if use_jit {
            let builder = JITBuilder::with_isa(target_isa, libcall_names);
            DynModule::Jit(cranelift_jit::JITModule::new(builder))
        } else {
            let builder = ObjectBuilder::new(target_isa, output_name, libcall_names);
            DynModule::Static(cranelift_object::ObjectModule::new(builder.unwrap()))
        };

        (module, frontend_config)
    }

    pub fn finish(self, main_id: FuncId, output_file: &Path) {
        match self {
            DynModule::Jit(mut module) => {
                module.finalize_definitions();
                let main = module.get_finalized_function(main_id);
                let main: fn() -> i32 = unsafe { std::mem::transmute(main) };
                main();
            },
            DynModule::Static(module) => {
                let product = module.finish();
                let text = product.object.write().unwrap();
                std::fs::write(output_file, text).unwrap();

                let executable = util::binary_name(output_file.to_string_lossy().as_ref());
                util::link(output_file.to_string_lossy().as_ref(), &executable);
            },
        }
    }
}

macro_rules! dispatch_on_module {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            DynModule::Jit(module) =>    $function(module $(, $($args),* )? ),
            DynModule::Static(module) => $function(module $(, $($args),* )? ),
        }
    });
}

impl Module for DynModule {
    fn isa(&self) -> &dyn cranelift::prelude::isa::TargetIsa {
        dispatch_on_module!(self, Module::isa)
    }

    fn declarations(&self) -> &cranelift_module::ModuleDeclarations {
        dispatch_on_module!(self, Module::declarations)
    }

    fn declare_function(
        &mut self, name: &str, linkage: Linkage, signature: &cranelift::codegen::ir::Signature,
    ) -> cranelift_module::ModuleResult<FuncId> {
        dispatch_on_module!(self, Module::declare_function, name, linkage, signature)
    }

    fn declare_anonymous_function(
        &mut self, signature: &cranelift::codegen::ir::Signature,
    ) -> cranelift_module::ModuleResult<FuncId> {
        dispatch_on_module!(self, Module::declare_anonymous_function, signature)
    }

    fn declare_data(
        &mut self, name: &str, linkage: Linkage, writable: bool, tls: bool,
    ) -> cranelift_module::ModuleResult<cranelift_module::DataId> {
        dispatch_on_module!(self, Module::declare_data, name, linkage, writable, tls)
    }

    fn declare_anonymous_data(
        &mut self, writable: bool, tls: bool,
    ) -> cranelift_module::ModuleResult<cranelift_module::DataId> {
        dispatch_on_module!(self, Module::declare_anonymous_data, writable, tls)
    }

    fn define_function(
        &mut self, func: FuncId, ctx: &mut cranelift::codegen::Context,
    ) -> cranelift_module::ModuleResult<cranelift_module::ModuleCompiledFunction> {
        dispatch_on_module!(self, Module::define_function, func, ctx)
    }

    fn define_function_bytes(
        &mut self, func: FuncId, bytes: &[u8], relocs: &[cranelift::codegen::MachReloc],
    ) -> cranelift_module::ModuleResult<cranelift_module::ModuleCompiledFunction> {
        dispatch_on_module!(self, Module::define_function_bytes, func, bytes, relocs)
    }

    fn define_data(
        &mut self, data: cranelift_module::DataId, data_ctx: &DataContext,
    ) -> cranelift_module::ModuleResult<()> {
        dispatch_on_module!(self, Module::define_data, data, data_ctx)
    }
}
