//! GPU backend initializer stub for fulgor::renderer::gpu_optional

pub fn initialize_gpu_backend() -> Result<(), String> {
    inner_initialize_gpu_backend()
}

#[cfg(not(feature = "gpu"))]
fn inner_initialize_gpu_backend() -> Result<(), String> {
    Err("gpu feature not enabled".into())
}

#[cfg(feature = "gpu")]
fn inner_initialize_gpu_backend() -> Result<(), String> {
    use wgpu::{Backends, Instance, InstanceDescriptor};

    let descriptor = InstanceDescriptor {
        backends: Backends::all(),
        dx12_shader_compiler: Default::default(),
    };

    let instance = Instance::new(descriptor);

    let mut found_adapter = false;
    for adapter in instance.enumerate_adapters(Backends::all()) {
        let info = adapter.get_info();
        println!(
            "fulgor::renderer::gpu_optional: adapter '{}', backend: {:?}, device: {:?}",
            info.name, info.backend, info.device
        );
        found_adapter = true;
        break;
    }

    if !found_adapter {
        println!("fulgor::renderer::gpu_optional: no adapters found");
    }

    Ok(())
}
