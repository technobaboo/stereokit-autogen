extern crate bindgen;

use bindgen::callbacks::{
	EnumVariantValue, ItemInfo, ItemKind, MacroParsingBehavior, ParseCallbacks,
};
use convert_case::{Case, Casing};
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct MacroCallback;
impl ParseCallbacks for MacroCallback {
	fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
		match name {
			"FP_NAN" => MacroParsingBehavior::Ignore,
			"FP_INFINITE" => MacroParsingBehavior::Ignore,
			"FP_ZERO" => MacroParsingBehavior::Ignore,
			"FP_SUBNORMAL" => MacroParsingBehavior::Ignore,
			"FP_NORMAL" => MacroParsingBehavior::Ignore,
			_ => MacroParsingBehavior::Default,
		}
	}
	fn generated_name_override(&self, item_info: ItemInfo<'_>) -> Option<String> {
		if let ItemKind::Var = item_info.kind {
			Some(item_info.name.to_case(Case::Pascal))
		} else {
			None
		}
	}
	fn item_name(&self, original_item_name: &str) -> Option<String> {
		if original_item_name.ends_with('_') {
			Some(original_item_name.to_case(Case::Pascal))
		} else {
			None
		}
	}
	fn enum_variant_name(
		&self,
		enum_name: Option<&str>,
		original_variant_name: &str,
		_variant_value: EnumVariantValue,
	) -> Option<String> {
		let mut name = original_variant_name.to_string();
		if let Some(enum_name) = enum_name {
			let enum_name = enum_name.trim_start_matches("enum ");
			// don't want DepthMode::DepthModeD32 because that's redundant!
			name = name.trim_start_matches(enum_name).to_string();
			// but rust won't let us make an enum value starting with a number so :/
			if name.starts_with(char::is_numeric) {
				name = dbg!(enum_name
					.trim_end_matches('_')
					.split('_')
					.last()
					.unwrap()
					.to_string()) + &name;
			}
		}
		Some(name.to_case(Case::Pascal))
	}
}

macro_rules! cargo_cmake_feat {
	($feature:literal) => {
		if cfg!(feature = $feature) {
			"ON"
		} else {
			"OFF"
		}
	};
}
macro_rules! cargo_link {
	($feature:expr) => {
		println!("cargo:rustc-link-lib={}", $feature);
	};
}
fn main() {
	let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
	let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();

	// Build StereoKit, and tell rustc to link it.
	let mut cmake_config = cmake::Config::new("StereoKit");
	cmake_config.define("SK_BUILD_SHARED_LIBS", "OFF");
	cmake_config.define("SK_BUILD_TESTS", "OFF");
	cmake_config.define("SK_LINUX_EGL", cargo_cmake_feat!("linux-egl"));
	cmake_config.define("SK_PHYSICS", cargo_cmake_feat!("physics")); // cannot get this to work on windows.
	if target_os == "android" {
		cmake_config.define("CMAKE_ANDROID_API", "25");
		//cmake_config.define("ANDROID", "TRUE");
	}

	let dst = cmake_config.build();

	println!("cargo:rustc-link-search=native={}/lib", dst.display());
	println!("cargo:rustc-link-search=native={}/lib64", dst.display());
	cargo_link!("static=StereoKitC");
	match target_family.as_str() {
		"windows" => {
			if cfg!(debug_assertions) {
				cargo_link!("static=openxr_loaderd");
			} else {
				cargo_link!("static=openxr_loader");
			}
			cargo_link!("windowsapp");
			cargo_link!("user32");
			cargo_link!("comdlg32");
			println!("cargo:rustc-link-search=native={}", dst.display());
			if cfg!(feature = "physics") {
				println!("cargo:rustc-link-lib=static=build/_deps/reactphysics3d-build/Debug/reactphysics3d");
			}
			//cargo_link!("static=reactphysics3d");
		}
		"wasm" => {
			unimplemented!("sorry wasm isn't implemented yet");
		}
		"unix" => {
			if target_os == "macos" {
				panic!("Sorry, macos is not supported for stereokit.");
			}
			cargo_link!("stdc++");
			cargo_link!("openxr_loader");
			if target_os == "android" {
				cargo_link!("android");
				cargo_link!("EGL");
			} else {
				cargo_link!("X11");
				cargo_link!("Xfixes");
				cargo_link!("GL");
				if cfg!(feature = "linux-egl") {
					cargo_link!("EGL");
					cargo_link!("gbm");
				} else {
					cargo_link!("GLEW");
					cargo_link!("GLX");
				}
				cargo_link!("fontconfig");
			}
		}
		_ => {
			panic!("target family is unknown");
		}
	}

	// Tell cargo to invalidate the built crate whenever the wrapper changes
	println!("cargo:rerun-if-changed=src/static-wrapper.h");
	println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit.h");
	println!("cargo:rerun-if-changed=StereoKit/StereoKitC/stereokit_ui.h");

	// On Android, we must ensure that we're dynamically linking against the C++ standard library.
	// For more details, see https://github.com/rust-windowing/android-ndk-rs/issues/167
	use std::env::var;
	if var("TARGET")
		.map(|target| target == "aarch64-linux-android")
		.unwrap_or(false)
	{
		// panic!("YO");
		println!("cargo:rustc-link-lib=dylib=c++");
	}

	// Generate bindings to StereoKitC.
	let bindings = bindgen::Builder::default()
		.header("src/static-wrapper.h")
		.blocklist_type("color128")
		.blocklist_type("color32")
		.blocklist_type("FP_NAN")
		.blocklist_type("FP_INFINITE")
		.blocklist_type("FP_ZERO")
		.blocklist_type("FP_SUBNORMAL")
		.blocklist_type("FP_NORMAL")
		.blocklist_function("_.*")
		// Blocklist functions with u128 in signature.
		// https://github.com/zmwangx/rust-ffmpeg-sys/issues/1
		// https://github.com/rust-lang/rust-bindgen/issues/1549
		.blocklist_function("acoshl")
		.blocklist_function("acosl")
		.blocklist_function("asinhl")
		.blocklist_function("asinl")
		.blocklist_function("atan2l")
		.blocklist_function("atanhl")
		.blocklist_function("atanl")
		.blocklist_function("cbrtl")
		.blocklist_function("ceill")
		.blocklist_function("copysignl")
		.blocklist_function("coshl")
		.blocklist_function("cosl")
		.blocklist_function("dreml")
		.blocklist_function("ecvt_r")
		.blocklist_function("erfcl")
		.blocklist_function("erfl")
		.blocklist_function("exp2l")
		.blocklist_function("expl")
		.blocklist_function("expm1l")
		.blocklist_function("fabsl")
		.blocklist_function("fcvt_r")
		.blocklist_function("fdiml")
		.blocklist_function("finitel")
		.blocklist_function("floorl")
		.blocklist_function("fmal")
		.blocklist_function("fmaxl")
		.blocklist_function("fminl")
		.blocklist_function("fmodl")
		.blocklist_function("frexpl")
		.blocklist_function("gammal")
		.blocklist_function("hypotl")
		.blocklist_function("ilogbl")
		.blocklist_function("isinfl")
		.blocklist_function("isnanl")
		.blocklist_function("j0l")
		.blocklist_function("j1l")
		.blocklist_function("jnl")
		.blocklist_function("ldexpl")
		.blocklist_function("lgammal")
		.blocklist_function("lgammal_r")
		.blocklist_function("llrintl")
		.blocklist_function("llroundl")
		.blocklist_function("log10l")
		.blocklist_function("log1pl")
		.blocklist_function("log2l")
		.blocklist_function("logbl")
		.blocklist_function("logl")
		.blocklist_function("lrintl")
		.blocklist_function("lroundl")
		.blocklist_function("modfl")
		.blocklist_function("nanl")
		.blocklist_function("nearbyintl")
		.blocklist_function("nextafterl")
		.blocklist_function("nexttoward")
		.blocklist_function("nexttowardf")
		.blocklist_function("nexttowardl")
		.blocklist_function("powl")
		.blocklist_function("qecvt")
		.blocklist_function("qecvt_r")
		.blocklist_function("qfcvt")
		.blocklist_function("qfcvt_r")
		.blocklist_function("qgcvt")
		.blocklist_function("remainderl")
		.blocklist_function("remquol")
		.blocklist_function("rintl")
		.blocklist_function("roundl")
		.blocklist_function("scalbl")
		.blocklist_function("scalblnl")
		.blocklist_function("scalbnl")
		.blocklist_function("significandl")
		.blocklist_function("sinhl")
		.blocklist_function("sinl")
		.blocklist_function("sqrtl")
		.blocklist_function("strtold")
		.blocklist_function("tanhl")
		.blocklist_function("tanl")
		.blocklist_function("tgammal")
		.blocklist_function("truncl")
		.blocklist_function("y0l")
		.blocklist_function("y1l")
		.blocklist_function("ynl")
		.generate_block(true)
		.prepend_enum_name(false)
		.rustified_enum(".*")
		.disable_name_namespacing()
		.parse_callbacks(Box::new(MacroCallback))
		.generate()
		.expect("Unable to generate bindings");

	// Write the bindings to the $OUT_DIR/bindings.rs file.
	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}
