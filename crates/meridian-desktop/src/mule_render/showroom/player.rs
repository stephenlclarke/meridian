//! Active SH171 artwork. The original detailed mesh is not the flight contact mesh.
use super::{EMPTY_PIECE, Piece, Triangle, Uniforms, gpu, medusa, shader, subtract, vector};
use crate::mule_render::{View, valid_pose, view_vector};
use meridian_core::docking::RigidPose;

pub(crate) struct PlayerMedusaPass {
    hardware: gpu::Hardware,
    vertex_count: u32,
    active: bool,
    srgb: bool,
}

fn flight_uniforms(model: RigidPose, view: View, srgb: bool) -> Result<Uniforms, String> {
    let [aspect, focal, near, far] = view.projection;
    if !valid_pose(model, 1e-5)
        || !valid_pose(view.camera, 1e-5)
        || !view.projection.iter().all(|value| value.is_finite())
        || aspect <= 0.0
        || focal <= 0.0
        || near <= 0.0
        || far <= near
    {
        return Err("Invalid Medusa player presentation frame".into());
    }
    let transform = |v| view_vector(view.flight, view.camera.orientation.inverse_coordinates(v));
    let relative = subtract(model.position, view.camera.position);
    let mut origin = vector(transform(relative));
    origin[3] = if srgb { 1.0 } else { 0.0 };
    Ok(Uniforms {
        projection: view.projection,
        origin,
        x: vector(transform(model.orientation.x)),
        y: vector(transform(model.orientation.y)),
        z: vector(transform(model.orientation.z)),
        camera: vector(
            model
                .orientation
                .inverse_coordinates(subtract(view.camera.position, model.position)),
        ),
        banks: [0.0; 4],
        low: [0.0; 4],
        high: [0.0; 4],
    })
}

impl PlayerMedusaPass {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        let vertices = medusa::visual()?;
        // Flight does not render the archived showroom shield. The solid material shader
        // shares its bind layout with inspection; only an inert, unused support is supplied.
        let points = [<Triangle as bytemuck::Zeroable>::zeroed()];
        let hardware = gpu::Hardware::new(
            device,
            queue,
            format,
            gpu::Inputs {
                shader: &shader(1),
                uniform_size: (size_of::<Uniforms>() + size_of::<Piece>()) as u64,
                points: &points,
                visual: &vertices,
                authored_materials: false,
                display_paint: true,
            },
        )?;
        queue.write_buffer(
            &hardware.uniform,
            size_of::<Uniforms>() as u64,
            bytemuck::bytes_of(&EMPTY_PIECE),
        );
        Ok(Self {
            hardware,
            vertex_count: vertices.len() as u32,
            active: false,
            srgb: format.is_srgb(),
        })
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        model: RigidPose,
        view: View,
    ) -> Result<(), String> {
        self.active = false;
        let data = flight_uniforms(model, view, self.srgb)?;
        queue.write_buffer(&self.hardware.uniform, 0, bytemuck::bytes_of(&data));
        self.active = true;
        Ok(())
    }

    pub(crate) fn hide(&mut self) {
        self.active = false;
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.active {
            pass.set_pipeline(&self.hardware.solid);
            pass.set_bind_group(0, &self.hardware.group, &[]);
            pass.set_vertex_buffer(0, self.hardware.vertices.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::{
        FlightView,
        docking::{Basis3, Vec3},
    };

    #[test]
    fn medusa_player_uniforms_follow_the_shared_flight_camera() {
        let model = RigidPose {
            position: Vec3::new(100.0, 200.0, 300.0),
            orientation: Basis3::IDENTITY,
        };
        let camera = RigidPose {
            position: Vec3::new(100.0, 215.0, 224.0),
            orientation: Basis3::IDENTITY,
        };
        let mut view = View {
            camera,
            flight: FlightView::Front,
            projection: [1.5, 1.6, 1.0, 20_000.0],
            tick: 0,
        };
        let uniform = flight_uniforms(model, view, true).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(uniform.origin, [0.0, -15.0, 76.0, 1.0]);
        assert_eq!(uniform.z, [0.0, 0.0, 1.0, 0.0]);
        view.flight = FlightView::Rear;
        let uniform = flight_uniforms(model, view, false).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(uniform.origin, [0.0, -15.0, -76.0, 0.0]);
        assert_eq!(uniform.z, [0.0, 0.0, -1.0, 0.0]);
    }
}
