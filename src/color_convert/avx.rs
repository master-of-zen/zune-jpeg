//! AVX color conversion routines
//!
//! Okay these codes are cool
//!
//! Herein lies super optimized codes to do color conversions.
//!
//!
//! 1. The YcbCr to RGB use integer approximations and not the floating point equivalent.
//! That means we may be +- 2 of pixels generated by libjpeg-turbo jpeg decoding
//! (also libjpeg uses routines like `Y  =  0.29900 * R + 0.33700 * G + 0.11400 * B + 0.25000 * G`)
//!
//! Firstly, we use integers (fun fact:there is no part of this code base where were dealing with
//! floating points.., fun fact: the first fun fact wasn't even fun.)
//!
//! Secondly ,we have cool clamping code, especially for rgba , where we don't need clamping and we
//! spend our time cursing that Intel decided permute instructions to work like 2 128 bit vectors(the compiler opitmizes
//! it out to something cool).
//!
//! There isn't a lot here (not as fun as bitstream ) but I hope you find what you're looking for.
//!
//! O and ~~subscribe to my youtube channel~~

#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#![cfg(feature = "x86")]
#![allow(
    clippy::wildcard_imports,
    clippy::cast_possible_truncation,
    clippy::too_many_arguments,
    clippy::inline_always,
    clippy::doc_markdown
)]

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub union YmmRegister
{
    // both are 32 when using std::mem::size_of
    mm256: __m256i,
    // for avx color conversion
    array: [i16; 16],
}

//--------------------------------------------------------------------------------------------------
// AVX conversion routines
//--------------------------------------------------------------------------------------------------

///
/// Convert YCBCR to RGB using AVX instructions
///
///  # Note
///**IT IS THE RESPONSIBILITY OF THE CALLER TO CALL THIS IN CPUS SUPPORTING
/// AVX2 OTHERWISE THIS IS UB**
///
/// *Peace*
///
/// This library itself will ensure that it's never called in CPU's not
/// supporting AVX2
///
/// # Arguments
/// - `y`,`cb`,`cr`: A reference of 8 i32's
/// - `out`: The output  array where we store our converted items
/// - `offset`: The position from 0 where we write these RGB values
#[inline(always)]
pub fn ycbcr_to_rgb_avx2(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16], out: &mut [u8], offset: &mut usize,
)
{
    // call this in another function to tell RUST to vectorize this
    // storing
    unsafe {
        ycbcr_to_rgb_avx2_1(y, cb, cr, out, offset);
    }
}

#[inline]
#[target_feature(enable = "avx2")]
#[target_feature(enable = "avx")]
unsafe fn ycbcr_to_rgb_avx2_1(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16], out: &mut [u8], offset: &mut usize,
)
{
    // Load output buffer
    let tmp: &mut [u8; 48] = out
        .get_mut(*offset..*offset + 48)
        .expect("Slice to small cannot write")
        .try_into()
        .unwrap();

    let (r, g, b) = ycbcr_to_rgb_baseline(y, cb, cr);

    let mut j = 0;
    let mut i = 0;
    while i < 48
    {
        tmp[i] = r.array[j] as u8;

        tmp[i + 1] = g.array[j] as u8;
        tmp[i + 2] = b.array[j] as u8;
        i += 3;
        j += 1;
    }

    *offset += 48;
}

/// Baseline implementation of YCBCR to RGB for avx,
///
/// It uses integer operations as opposed to floats, the approximation is
/// difficult for the  eye to see, but this means that it may produce different
/// values with libjpeg_turbo.  if accuracy is of utmost importance, use that.
///
/// this function should be called for most implementations, including
/// - ycbcr->rgb
/// - ycbcr->rgba
/// - ycbcr->brga
/// - ycbcr->rgbx
#[inline]
#[target_feature(enable = "avx2")]
#[target_feature(enable = "avx")]
unsafe fn ycbcr_to_rgb_baseline(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16],
) -> (YmmRegister, YmmRegister, YmmRegister)
{
    // Load values into a register
    //
    // dst[127:0] := MEM[loaddr+127:loaddr]
    // dst[255:128] := MEM[hiaddr+127:hiaddr]
    let y_c = _mm256_loadu_si256(y.as_ptr().cast());

    let cb_c = _mm256_loadu_si256(cb.as_ptr().cast());

    let cr_c = _mm256_loadu_si256(cr.as_ptr().cast());

    // AVX version of integer version in https://stackoverflow.com/questions/4041840/function-to-convert-ycbcr-to-rgb

    // Cb = Cb-128;
    let cb_r = _mm256_sub_epi16(cb_c, _mm256_set1_epi16(128));

    // cr = Cb -128;
    let cr_r = _mm256_sub_epi16(cr_c, _mm256_set1_epi16(128));

    // Calculate Y->R
    // r = Y + 45 * Cr / 32
    // 45*cr
    let r1 = _mm256_mullo_epi16(_mm256_set1_epi16(45), cr_r);

    // r1>>5
    let r2 = _mm256_srai_epi16::<5>(r1);

    //y+r2

    let r = YmmRegister {
        mm256: clamp_avx(_mm256_add_epi16(y_c, r2)),
    };

    // g = Y - (11 * Cb + 23 * Cr) / 32 ;

    // 11*cb
    let g1 = _mm256_mullo_epi16(_mm256_set1_epi16(11), cb_r);

    // 23*cr
    let g2 = _mm256_mullo_epi16(_mm256_set1_epi16(23), cr_r);

    //(11
    //(11 * Cb + 23 * Cr)
    let g3 = _mm256_add_epi16(g1, g2);

    // (11 * Cb + 23 * Cr) / 32
    let g4 = _mm256_srai_epi16::<5>(g3);

    // Y - (11 * Cb + 23 * Cr) / 32 ;
    let g = YmmRegister {
        mm256: clamp_avx(_mm256_sub_epi16(y_c, g4)),
    };

    // b = Y + 113 * Cb / 64
    // 113 * cb
    let b1 = _mm256_mullo_epi16(_mm256_set1_epi16(113), cb_r);

    //113 * Cb / 64
    let b2 = _mm256_srai_epi16::<6>(b1);

    // b = Y + 113 * Cb / 64 ;
    let b = YmmRegister {
        mm256: clamp_avx(_mm256_add_epi16(b2, y_c)),
    };

    return (r, g, b);
}

#[inline]
#[target_feature(enable = "avx2")]
/// A baseline implementation of YCbCr to RGB conversion which does not carry
/// out clamping
///
/// This is used by the `ycbcr_to_rgba_avx` and `ycbcr_to_rgbx` conversion
/// routines
unsafe fn ycbcr_to_rgb_baseline_no_clamp(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16],
) -> (__m256i, __m256i, __m256i)
{
    // Load values into a register
    //
    let y_c = _mm256_loadu_si256(y.as_ptr().cast());

    let cb_c = _mm256_loadu_si256(cb.as_ptr().cast());

    let cr_c = _mm256_loadu_si256(cr.as_ptr().cast());

    // AVX version of integer version in https://stackoverflow.com/questions/4041840/function-to-convert-ycbcr-to-rgb

    // Cb = Cb-128;
    let cb_r = _mm256_sub_epi16(cb_c, _mm256_set1_epi16(128));

    // cr = Cb -128;
    let cr_r = _mm256_sub_epi16(cr_c, _mm256_set1_epi16(128));

    // Calculate Y->R
    // r = Y + 45 * Cr / 32
    // 45*cr
    let r1 = _mm256_mullo_epi16(_mm256_set1_epi16(45), cr_r);

    // r1>>5
    let r2 = _mm256_srai_epi16::<5>(r1);

    //y+r2

    let r = _mm256_add_epi16(y_c, r2);

    // g = Y - (11 * Cb + 23 * Cr) / 32 ;

    // 11*cb
    let g1 = _mm256_mullo_epi16(_mm256_set1_epi16(11), cb_r);

    // 23*cr
    let g2 = _mm256_mullo_epi16(_mm256_set1_epi16(23), cr_r);

    //(11
    //(11 * Cb + 23 * Cr)
    let g3 = _mm256_add_epi16(g1, g2);

    // (11 * Cb + 23 * Cr) / 32
    let g4 = _mm256_srai_epi16::<5>(g3);

    // Y - (11 * Cb + 23 * Cr) / 32 ;
    let g = _mm256_sub_epi16(y_c, g4);

    // b = Y + 113 * Cb / 64
    // 113 * cb
    let b1 = _mm256_mullo_epi16(_mm256_set1_epi16(113), cb_r);

    //113 * Cb / 64
    let b2 = _mm256_srai_epi16::<6>(b1);

    // b = Y + 113 * Cb / 64 ;
    let b = _mm256_add_epi16(b2, y_c);

    return (r, g, b);
}

#[inline(always)]
pub fn ycbcr_to_rgba_avx2(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16], out: &mut [u8], offset: &mut usize,
)
{
    unsafe {
        ycbcr_to_rgba_unsafe(y, cb, cr, out, offset);
    }
}

#[inline]
#[target_feature(enable = "avx2")]
#[rustfmt::skip]
unsafe fn ycbcr_to_rgba_unsafe(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16],
    out: &mut [u8],
    offset: &mut usize,
)
{
    // check if we have enough space to write.
    let tmp:& mut [u8; 64] = out.get_mut(*offset..*offset + 64).expect("Slice to small cannot write").try_into().unwrap();

    let (r, g, b) = ycbcr_to_rgb_baseline_no_clamp(y, cb, cr);

    // set alpha channel to 255 for opaque

    // And no these comments were not from me pressing the keyboard

    // Pack the integers into u8's using signed saturation.
    let c = _mm256_packus_epi16(r, g); //aaaaa_bbbbb_aaaaa_bbbbbb
    let d = _mm256_packus_epi16(b, _mm256_set1_epi16(255)); // cccccc_dddddd_ccccccc_ddddd
    // transpose and interleave channels
    let e = _mm256_unpacklo_epi8(c, d); //ab_ab_ab_ab_ab_ab_ab_ab
    let f = _mm256_unpackhi_epi8(c, d); //cd_cd_cd_cd_cd_cd_cd_cd
    // final transpose
    let g = _mm256_unpacklo_epi8(e, f); //abcd_abcd_abcd_abcd_abcd
    let h = _mm256_unpackhi_epi8(e, f);


    // undo packus shuffling...
    let i = _mm256_permute2x128_si256::<{ shuffle(3, 2, 1, 0) }>(g, h);

    let j = _mm256_permute2x128_si256::<{ shuffle(1, 2, 3, 0) }>(g, h);

    let k = _mm256_permute2x128_si256::<{ shuffle(3, 2, 0, 1) }>(g, h);

    let l = _mm256_permute2x128_si256::<{ shuffle(0, 3, 2, 1) }>(g, h);

    let m = _mm256_blend_epi32::<0b1111_0000>(i, j);

    let n = _mm256_blend_epi32::<0b1111_0000>(k, l);


    // Store
    // Use streaming instructions to prevent polluting the cache?
    _mm256_storeu_si256(tmp.as_mut_ptr().cast(), m);

    _mm256_storeu_si256(tmp[32..].as_mut_ptr().cast(), n);

    *offset += 64;
}

/// YCbCr to RGBX conversion
///
/// The X in RGBX stands for `anything`, the compiler will make X anything it
/// sees fit, although most implementations use
///
/// This is meant to match libjpeg-turbo RGBX conversion and since its
/// a 4 way interleave instead of a three way interleave, the code is simple
/// to vectorize hence this is faster than YcbCr -> RGB conversion
#[inline(always)]
pub fn ycbcr_to_rgbx_avx2(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16], out: &mut [u8], offset: &mut usize,
)
{
    unsafe {
        ycbcr_to_rgbx_unsafe(y, cb, cr, out, offset);
    }
}

#[inline]
#[allow(clippy::cast_possible_wrap)]
#[target_feature(enable = "avx2")]
#[rustfmt::skip]
unsafe fn ycbcr_to_rgbx_unsafe(
    y: &[i16; 16], cb: &[i16; 16], cr: &[i16; 16],
    out: &mut [u8],
    offset: &mut usize,
)
{
    let tmp:& mut [u8; 64] = out.get_mut(*offset..*offset + 64).expect("Slice to small cannot write").try_into().unwrap();

    let (r, g, b) = ycbcr_to_rgb_baseline_no_clamp(y, cb, cr);

    // Pack the integers into u8's using signed saturation.
    let c = _mm256_packus_epi16(r, g); //aaaaa_bbbbb_aaaaa_bbbbbb
    // Set alpha channel to random things, Mostly I see it using the b values
    let d = _mm256_packus_epi16(b, _mm256_undefined_si256()); // cccccc_dddddd_ccccccc_ddddd
    // transpose and interleave channels
    let e = _mm256_unpacklo_epi8(c, d); //ab_ab_ab_ab_ab_ab_ab_ab
    let f = _mm256_unpackhi_epi8(c, d); //cd_cd_cd_cd_cd_cd_cd_cd
    // final transpose
    let g = _mm256_unpacklo_epi8(e, f); //abcd_abcd_abcd_abcd_abcd
    let h = _mm256_unpackhi_epi8(e, f);

    // undo packus shuffling...
    // This only applies to AVX because it's packus is a bit weird
    // and PLEASE DO NOT CHANGE SHUFFLE CONSTANTS.
    // COMING UP WITH THEM TOOK SOO LONG...
    let i = _mm256_permute2x128_si256::<{ shuffle(3, 2, 1, 0) }>(g, h);

    let j = _mm256_permute2x128_si256::<{ shuffle(1, 2, 3, 0) }>(g, h);

    let k = _mm256_permute2x128_si256::<{ shuffle(3, 2, 0, 1) }>(g, h);

    let l = _mm256_permute2x128_si256::<{ shuffle(0, 3, 2, 1) }>(g, h);

    let m = _mm256_blend_epi32::<0b1111_0000>(i, j);

    let n = _mm256_blend_epi32::<0b1111_0000>(k, l);

    // check if we have enough space to write.
    // Store
    // Use streaming instructions to prevent polluting the cache
    _mm256_storeu_si256(tmp.as_mut_ptr().cast(), m);

    _mm256_storeu_si256(tmp[32..].as_mut_ptr().cast(), n);

    *offset += 64;
}

/// Clamp values between 0 and 255
///
/// This function clamps all values in `reg` to be between 0 and 255
///( the accepted values for RGB)
#[inline]
#[target_feature(enable = "avx2")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn clamp_avx(reg: __m256i) -> __m256i
{
    // the lowest value
    let min_s = _mm256_set1_epi16(0);

    // Highest value
    let max_s = _mm256_set1_epi16(255);

    let max_v = _mm256_max_epi16(reg, min_s); //max(a,0)
    let min_v = _mm256_min_epi16(max_v, max_s); //min(max(a,0),255)
    return min_v;
}

#[inline]
const fn shuffle(z: i32, y: i32, x: i32, w: i32) -> i32
{
    ((z << 6) | (y << 4) | (x << 2) | w) as i32
}
