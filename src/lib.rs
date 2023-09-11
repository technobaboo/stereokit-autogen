#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{any::Any, ffi::c_void, marker::PhantomData, panic::AssertUnwindSafe};

mod conversions;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// #[repr(C)]
// #[derive(Debug, Copy, Clone, Default)]
// #[cfg_attr(feature = "bevy_ecs", derive(bevy_ecs::prelude::Component))]
// #[cfg_attr(
// 	feature = "bevy_reflect",
// 	derive(bevy_reflect::prelude::Reflect, bevy_reflect::prelude::FromReflect)
// )]
// #[cfg_attr(feature = "bevy_reflect", reflect(Component))]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// pub struct color128 {
// 	pub r: f32,
// 	pub g: f32,
// 	pub b: f32,
// 	pub a: f32,
// }
// #[cfg(feature = "bevy_ecs")]
// use bevy_ecs::prelude::ReflectComponent;
// #[cfg(feature = "serde")]
// use serde::{Deserialize, Serialize};

// #[repr(C)]
// #[derive(Debug, Copy, Clone, Default)]
// #[cfg_attr(feature = "bevy_ecs", derive(bevy_ecs::prelude::Component))]
// #[cfg_attr(
// 	feature = "bevy_reflect",
// 	derive(bevy_reflect::prelude::Reflect, bevy_reflect::prelude::FromReflect)
// )]
// #[cfg_attr(feature = "bevy_reflect", reflect(Component))]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// pub struct color32 {
// 	pub r: u8,
// 	pub g: u8,
// 	pub b: u8,
// 	pub a: u8,
// }

// impl Clone for sh_light_t {
// 	fn clone(&self) -> Self {
// 		*self
// 	}
// }

// impl Copy for sh_light_t {}

// impl Clone for gradient_key_t {
// 	fn clone(&self) -> Self {
// 		*self
// 	}
// }

// impl Copy for gradient_key_t {}

// impl Clone for vert_t {
// 	fn clone(&self) -> Self {
// 		*self
// 	}
// }

// impl Copy for vert_t {}

// impl Clone for line_point_t {
// 	fn clone(&self) -> Self {
// 		*self
// 	}
// }

// impl Copy for line_point_t {}

// unsafe impl Sync for material_t {}
// unsafe impl Send for material_t {}
// unsafe impl Sync for model_t {}
// unsafe impl Send for model_t {}
// unsafe impl Sync for tex_t {}
// unsafe impl Send for tex_t {}
// unsafe impl Sync for sound_t {}
// unsafe impl Send for sound_t {}
// unsafe impl Sync for material_buffer_t {}
// unsafe impl Send for material_buffer_t {}
// unsafe impl Sync for sprite_t {}
// unsafe impl Send for sprite_t {}
// unsafe impl Sync for font_t {}
// unsafe impl Send for font_t {}
// unsafe impl Sync for gradient_t {}
// unsafe impl Send for gradient_t {}
// unsafe impl Sync for shader_t {}
// unsafe impl Send for shader_t {}

// impl AsRef<bool> for Bool32 {
// 	fn as_ref(&self) -> &bool {
// 		&(self.0 == 1)
// 	}
// }

#[derive(Clone)]
pub struct SkMultiThreaded(pub(crate) PhantomData<*const ()>);
impl SkMultiThreaded {
	/// only use if you know what you are doing
	pub unsafe fn create_unsafe() -> Self {
		SkMultiThreaded(PhantomData)
	}
}

pub struct SkSingleThreaded(SkMultiThreaded);
impl AsRef<SkMultiThreaded> for SkSingleThreaded {
	fn as_ref(&self) -> &SkMultiThreaded {
		&self.0
	}
}
impl SkSingleThreaded {
	/// only use if you know what you are doing
	pub unsafe fn create_unsafe() -> Self {
		SkSingleThreaded(SkMultiThreaded(PhantomData))
	}
	pub fn multithreaded(&self) -> SkMultiThreaded {
		self.0.clone()
	}
}

type PanicPayload = Box<dyn Any + Send + 'static>;

/// SAFETY: payload_ptr must point to a value of type
/// `(&mut F, LST, GST, &mut Option<PanicPayload>)`.
/// It must also not be called synchronously with itself
/// or any other callback using the same parameters (due to &mut).
/// If `caught_panic` is written to, `F` and `LST` are
/// panic-poisoned, and the panic should likely be propagated.
unsafe extern "C" fn callback_trampoline<F, LST, GST>(payload_ptr: *mut c_void)
where
	F: FnMut(&mut LST, &mut GST),
{
	let payload =
		&mut *(payload_ptr as *mut (&mut F, &mut LST, &mut GST, &mut Option<PanicPayload>));
	let (closure, state, global_state, caught_panic) = payload;

	if caught_panic.is_some() {
		// we should consider the state poisoned and not run the callback
		return;
	}

	// the caller should ensure closure variables and state cannot be observed
	// after the panic without catching the panic,
	// which will in turn require them to be UnwindSafe
	let mut closure = AssertUnwindSafe(closure);
	let mut state = AssertUnwindSafe(state);
	// TODO: is global state always safe to be re-observed after a shutdown?
	let mut global_state = AssertUnwindSafe(global_state);

	let result = std::panic::catch_unwind(move || closure(*state, *global_state));
	if let Err(panic_payload) = result {
		caught_panic.replace(panic_payload);
		sk_quit();
	}
}

// static mut GLOBAL_THING: Option<Box<dyn FnMut(&CSkDraw)>> = None;
// static mut wait_for_me: bool = false;
//
// extern "C" fn private_sk_step_func() {
//     unsafe {
//         GLOBAL_THING.as_mut().unwrap()(&CSkDraw(PhantomData));
//         wait_for_me = false;
//     }
// }

impl SkSingleThreaded {
	// pub fn step(&mut self, mut on_step: impl FnMut(&CSkDraw) + 'static) {
	//     unsafe {
	//         while wait_for_me {}
	//         GLOBAL_THING.replace(Box::new(on_step));
	//         wait_for_me = true;
	//         stereokit_sys::sk_step(Some(private_sk_step_func));
	//     }
	// }
	pub fn run(
		self,
		mut on_update: impl FnMut(&SkSingleThreaded),
		mut on_close: impl FnMut(&SkSingleThreaded),
	) {
		self.run_stateful(&mut (), |_, sk| on_update(sk), |_, sk| on_close(sk));
	}

	fn run_stateful<ST, U, S>(mut self, state: &mut ST, mut update: U, mut shutdown: S)
	where
		U: FnMut(&mut ST, &mut SkSingleThreaded),
		S: FnMut(&mut ST, &mut SkSingleThreaded),
	{
		// use one variable so shutdown doesn't run if update panics
		let mut caught_panic = Option::<PanicPayload>::None;

		let mut update_ref: (
			&mut U,
			&mut ST,
			&mut SkSingleThreaded,
			&mut Option<PanicPayload>,
		) = (&mut update, state, &mut self, &mut caught_panic);
		let update_raw = &mut update_ref
			as *mut (
				&mut U,
				&mut ST,
				&mut SkSingleThreaded,
				&mut Option<PanicPayload>,
			) as *mut c_void;

		let mut shutdown_ref: (
			&mut S,
			&mut ST,
			&mut SkSingleThreaded,
			&mut Option<PanicPayload>,
		) = (&mut shutdown, state, &mut self, &mut caught_panic);
		let shutdown_raw = &mut shutdown_ref
			as *mut (
				&mut S,
				&mut ST,
				&mut SkSingleThreaded,
				&mut Option<PanicPayload>,
			) as *mut c_void;

		// if self.ran.set(()).is_err() {
		//     return;
		// }

		unsafe {
			sk_run_data(
				Some(callback_trampoline::<U, ST, SkSingleThreaded>),
				update_raw,
				Some(callback_trampoline::<S, ST, SkSingleThreaded>),
				shutdown_raw,
			);
		}

		if let Some(panic_payload) = caught_panic {
			std::panic::resume_unwind(panic_payload);
		}
	}
}
