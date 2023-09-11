use std::ptr;

use stereokit_sys::*;
fn main() {
	unsafe {
		if sk_init(SkSettings {
			app_name: ptr::null(),
			assets_folder: ptr::null(),
			display_preference: DisplayMode::Mixedreality,
			blend_preference: DisplayBlend::AnyTransparent,
			no_flatscreen_fallback: false.into(),
			depth_mode: DepthMode::D32,
			log_filter: Log::Diagnostic,
			overlay_app: false.into(),
			overlay_priority: 0,
			origin: OriginMode::Floor,
			flatscreen_pos_x: 0,
			flatscreen_pos_y: 0,
			flatscreen_width: 0,
			flatscreen_height: 0,
			disable_flatscreen_mr_sim: false.into(),
			disable_desktop_input_window: false.into(),
			disable_unfocused_sleep: false.into(),
			render_scaling: 1.0,
			render_multisample: 0,
			android_java_vm: ptr::null_mut(),
			android_activity: ptr::null_mut(),
		})
		.into()
		{
			panic!("Unable to initialize StereoKit");
		}

		sk_run(Some(step), None);
	}
}

unsafe extern "C" fn step() {
	mesh_draw(
		mesh_find(default_id_mesh_cube.as_ptr()),
		material_find(default_id_material_ui_box.as_ptr()),
		matrix_ts(
			vec3 {
				x: 0.0,
				y: 0.0,
				z: -0.5,
			},
			vec3 {
				x: 0.1,
				y: 0.1,
				z: 0.1,
			},
		),
		color128 {
			r: 1.0,
			g: 1.0,
			b: 1.0,
			a: 1.0,
		},
		RenderLayer::Layer0,
	);
}
