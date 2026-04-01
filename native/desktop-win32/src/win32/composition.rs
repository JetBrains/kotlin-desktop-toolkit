#![allow(clippy::inline_always)]
#![allow(clippy::ref_as_ptr)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use std::cell::RefCell;

use windows::{
    Foundation::{IPropertyValue, PropertyValue},
    Graphics::Effects::{IGraphicsEffect, IGraphicsEffect_Impl, IGraphicsEffectSource, IGraphicsEffectSource_Impl},
    Win32::{
        Foundation::{E_BOUNDS, E_INVALIDARG, E_POINTER},
        Globalization::{CSTR_EQUAL, CompareStringOrdinal},
        Graphics::Direct2D::{
            CLSID_D2D1Blend,
            Common::{
                D2D1_BLEND_MODE_COLOR, D2D1_BLEND_MODE_COLOR_BURN, D2D1_BLEND_MODE_COLOR_DODGE, D2D1_BLEND_MODE_DARKEN,
                D2D1_BLEND_MODE_DARKER_COLOR, D2D1_BLEND_MODE_DIFFERENCE, D2D1_BLEND_MODE_DISSOLVE, D2D1_BLEND_MODE_DIVISION,
                D2D1_BLEND_MODE_EXCLUSION, D2D1_BLEND_MODE_HARD_LIGHT, D2D1_BLEND_MODE_HARD_MIX, D2D1_BLEND_MODE_HUE,
                D2D1_BLEND_MODE_LIGHTEN, D2D1_BLEND_MODE_LIGHTER_COLOR, D2D1_BLEND_MODE_LINEAR_BURN, D2D1_BLEND_MODE_LINEAR_DODGE,
                D2D1_BLEND_MODE_LINEAR_LIGHT, D2D1_BLEND_MODE_LUMINOSITY, D2D1_BLEND_MODE_MULTIPLY, D2D1_BLEND_MODE_OVERLAY,
                D2D1_BLEND_MODE_PIN_LIGHT, D2D1_BLEND_MODE_SATURATION, D2D1_BLEND_MODE_SCREEN, D2D1_BLEND_MODE_SOFT_LIGHT,
                D2D1_BLEND_MODE_SUBTRACT, D2D1_BLEND_MODE_VIVID_LIGHT,
            },
            D2D1_BLEND_PROP_MODE,
        },
        System::WinRT::Graphics::Direct2D::{
            GRAPHICS_EFFECT_PROPERTY_MAPPING, GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT, IGraphicsEffectD2D1Interop,
            IGraphicsEffectD2D1Interop_Impl,
        },
    },
    core::{GUID, HSTRING, Interface, PCWSTR, Result as WinResult, implement, w},
};

#[implement(IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectD2D1Interop)]
pub(crate) struct BlendEffect {
    name: RefCell<HSTRING>,
    mode: BlendEffectMode,
    background: Option<IGraphicsEffectSource>,
    foreground: Option<IGraphicsEffectSource>,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlendEffectMode {
    Multiply = D2D1_BLEND_MODE_MULTIPLY.0.cast_unsigned(),
    Screen = D2D1_BLEND_MODE_SCREEN.0.cast_unsigned(),
    Darken = D2D1_BLEND_MODE_DARKEN.0.cast_unsigned(),
    Lighten = D2D1_BLEND_MODE_LIGHTEN.0.cast_unsigned(),
    Dissolve = D2D1_BLEND_MODE_DISSOLVE.0.cast_unsigned(),
    ColorBurn = D2D1_BLEND_MODE_COLOR_BURN.0.cast_unsigned(),
    LinearBurn = D2D1_BLEND_MODE_LINEAR_BURN.0.cast_unsigned(),
    DarkerColor = D2D1_BLEND_MODE_DARKER_COLOR.0.cast_unsigned(),
    LighterColor = D2D1_BLEND_MODE_LIGHTER_COLOR.0.cast_unsigned(),
    ColorDodge = D2D1_BLEND_MODE_COLOR_DODGE.0.cast_unsigned(),
    LinearDodge = D2D1_BLEND_MODE_LINEAR_DODGE.0.cast_unsigned(),
    Overlay = D2D1_BLEND_MODE_OVERLAY.0.cast_unsigned(),
    SoftLight = D2D1_BLEND_MODE_SOFT_LIGHT.0.cast_unsigned(),
    HardLight = D2D1_BLEND_MODE_HARD_LIGHT.0.cast_unsigned(),
    VividLight = D2D1_BLEND_MODE_VIVID_LIGHT.0.cast_unsigned(),
    LinearLight = D2D1_BLEND_MODE_LINEAR_LIGHT.0.cast_unsigned(),
    PinLight = D2D1_BLEND_MODE_PIN_LIGHT.0.cast_unsigned(),
    HardMix = D2D1_BLEND_MODE_HARD_MIX.0.cast_unsigned(),
    Difference = D2D1_BLEND_MODE_DIFFERENCE.0.cast_unsigned(),
    Exclusion = D2D1_BLEND_MODE_EXCLUSION.0.cast_unsigned(),
    Hue = D2D1_BLEND_MODE_HUE.0.cast_unsigned(),
    Saturation = D2D1_BLEND_MODE_SATURATION.0.cast_unsigned(),
    Color = D2D1_BLEND_MODE_COLOR.0.cast_unsigned(),
    Luminosity = D2D1_BLEND_MODE_LUMINOSITY.0.cast_unsigned(),
    Subtract = D2D1_BLEND_MODE_SUBTRACT.0.cast_unsigned(),
    Division = D2D1_BLEND_MODE_DIVISION.0.cast_unsigned(),
}

impl BlendEffect {
    pub fn new(name: &HSTRING) -> Self {
        Self {
            name: RefCell::new(name.to_owned()),
            mode: BlendEffectMode::Multiply,
            background: None,
            foreground: None,
        }
    }

    pub const fn set_mode(&mut self, value: BlendEffectMode) {
        self.mode = value;
    }

    pub const fn background(&self) -> Option<&IGraphicsEffectSource> {
        self.background.as_ref()
    }

    pub fn set_background(&mut self, value: IGraphicsEffectSource) {
        self.background.replace(value);
    }

    pub const fn foreground(&self) -> Option<&IGraphicsEffectSource> {
        self.foreground.as_ref()
    }

    pub fn set_foreground(&mut self, value: IGraphicsEffectSource) {
        self.foreground.replace(value);
    }
}

impl IGraphicsEffect_Impl for BlendEffect_Impl {
    fn Name(&self) -> WinResult<HSTRING> {
        Ok(self.name.borrow().clone())
    }

    fn SetName(&self, name: &HSTRING) -> WinResult<()> {
        self.name.replace(name.to_owned());
        Ok(())
    }
}

impl IGraphicsEffectSource_Impl for BlendEffect_Impl {}

impl IGraphicsEffectD2D1Interop_Impl for BlendEffect_Impl {
    fn GetEffectId(&self) -> WinResult<GUID> {
        Ok(CLSID_D2D1Blend)
    }

    fn GetNamedPropertyMapping(&self, name: &PCWSTR, index: *mut u32, mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING) -> WinResult<()> {
        if index.is_null() || mapping.is_null() {
            return Err(E_POINTER.into());
        }
        if unsafe { CompareStringOrdinal(name.as_wide(), w!("Mode").as_wide(), true) } != CSTR_EQUAL {
            return Err(E_INVALIDARG.into());
        }
        unsafe {
            index.write(D2D1_BLEND_PROP_MODE.0.cast_unsigned());
            mapping.write(GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT);
        }
        Ok(())
    }

    fn GetPropertyCount(&self) -> WinResult<u32> {
        Ok(1)
    }

    fn GetProperty(&self, index: u32) -> WinResult<IPropertyValue> {
        if index != D2D1_BLEND_PROP_MODE.0.cast_unsigned() {
            return Err(E_BOUNDS.into());
        }
        PropertyValue::CreateUInt32(self.mode as u32)?.cast()
    }

    fn GetSource(&self, index: u32) -> WinResult<IGraphicsEffectSource> {
        match index {
            0 => self.background().cloned().ok_or_else(|| E_POINTER.into()),
            1 => self.foreground().cloned().ok_or_else(|| E_POINTER.into()),
            _ => Err(E_BOUNDS.into()),
        }
    }

    fn GetSourceCount(&self) -> WinResult<u32> {
        Ok(2)
    }
}
