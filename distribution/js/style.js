const CHANNEL = {
    accent_yellow: [250, 245, 0],
    white: [255, 255, 255],
    black: [0, 0, 0],
};

const CONTRAST_ENHANCED = 7;
const CONTRAST_MINIMUM = 4.5;
const CONTRAST_GRAPHICAL = 3;

const FAMILY = {
    light: {
        "light": { paper: CHANNEL.white, text_contrast: CONTRAST_MINIMUM, highlight_paper: CHANNEL.accent_yellow },
        "light-more-contrast": { paper: CHANNEL.white, text_contrast: CONTRAST_ENHANCED, highlight_paper: CHANNEL.accent_yellow },
        "light-less-contrast": { paper: [243, 243, 243], text_contrast: CONTRAST_MINIMUM, highlight_paper: CHANNEL.accent_yellow },
    },
    dark: {
        "dark": { paper: CHANNEL.black, text_contrast: CONTRAST_MINIMUM, highlight_paper: mix_rgb(CHANNEL.accent_yellow, CHANNEL.black, 40.8) },
        "dark-more-contrast": { paper: CHANNEL.black, text_contrast: CONTRAST_ENHANCED, highlight_paper: mix_rgb(CHANNEL.accent_yellow, CHANNEL.black, 37.3) },
        "dark-less-contrast": { paper: [43, 43, 43], text_contrast: CONTRAST_MINIMUM, highlight_paper: mix_rgb(CHANNEL.accent_yellow, CHANNEL.black, 40.8) },
    },
};

const SOURCE_LIGHTNESS = 0.65;
const GRID_RESOLUTION = 1000;
const GRID_CHROMA_MAXIMUM = 0.4;
const GAMUT_TOLERANCE = 1e-7;
const SOURCE_CHROMA_STEP = 0.002;

function calculate_relative_luminance(rgb) {
    const [red, green, blue] = rgb.map((channel) => {
        const unit = channel / 255;
        return unit <= 0.04045 ? unit / 12.92 : Math.pow((unit + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function calculate_contrast(luminance, other_luminance) {
    const [lighter, darker] = luminance >= other_luminance ? [luminance, other_luminance] : [other_luminance, luminance];
    return (lighter + 0.05) / (darker + 0.05);
}

function mix_rgb(rgb, other_rgb, percent) {
    const weight = percent / 100;
    return [0, 1, 2].map((index) => Math.round(rgb[index] * weight + other_rgb[index] * (1 - weight)));
}

function convert_to_oklch(rgb) {
    const [red, green, blue] = rgb.map((channel) => {
        const unit = channel / 255;
        return unit <= 0.04045 ? unit / 12.92 : Math.pow((unit + 0.055) / 1.055, 2.4);
    });
    const cone_long = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue);
    const cone_medium = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue);
    const cone_short = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue);
    const green_red_axis = 1.9779984951 * cone_long - 2.4285922050 * cone_medium + 0.4505937099 * cone_short;
    const blue_yellow_axis = 0.0259040371 * cone_long + 0.7827717662 * cone_medium - 0.8086757660 * cone_short;
    return [
        0.2104542553 * cone_long + 0.7936177850 * cone_medium - 0.0040720468 * cone_short,
        Math.hypot(green_red_axis, blue_yellow_axis),
        (Math.atan2(blue_yellow_axis, green_red_axis) * 180 / Math.PI + 360) % 360,
    ];
}

function convert_to_rgb(lightness, chroma, hue) {
    const green_red_axis = chroma * Math.cos(hue * Math.PI / 180);
    const blue_yellow_axis = chroma * Math.sin(hue * Math.PI / 180);
    const cone_long = (lightness + 0.3963377774 * green_red_axis + 0.2158037573 * blue_yellow_axis) ** 3;
    const cone_medium = (lightness - 0.1055613458 * green_red_axis - 0.0638541728 * blue_yellow_axis) ** 3;
    const cone_short = (lightness - 0.0894841775 * green_red_axis - 1.2914855480 * blue_yellow_axis) ** 3;
    const linear = [
        4.0767416621 * cone_long - 3.3077115913 * cone_medium + 0.2309699292 * cone_short,
        -1.2684380046 * cone_long + 2.6097574011 * cone_medium - 0.3413193965 * cone_short,
        -0.0041960863 * cone_long - 0.7034186147 * cone_medium + 1.7076147010 * cone_short,
    ];
    if (linear.some((value) => value < -GAMUT_TOLERANCE || value > 1 + GAMUT_TOLERANCE)) return null;
    return linear.map((value) => {
        const unit = Math.min(1, Math.max(0, value));
        return Math.round((unit <= 0.0031308 ? 12.92 * unit : 1.055 * Math.pow(unit, 1 / 2.4) - 0.055) * 255);
    });
}

function find_vivid_rgb(lightness, hue) {
    let vivid = null;
    for (let chroma = 0; chroma <= GRID_CHROMA_MAXIMUM; chroma += SOURCE_CHROMA_STEP) {
        vivid = convert_to_rgb(lightness, chroma, hue) ?? vivid;
    }
    return vivid;
}

function build_grid(source_rgb) {
    const [source_lightness, source_chroma, hue] = convert_to_oklch(source_rgb);
    const chroma_step_count = Math.round(GRID_CHROMA_MAXIMUM * GRID_RESOLUTION);
    const capacity = (GRID_RESOLUTION + 1) * (chroma_step_count + 1) + 1;
    const lightness = new Float64Array(capacity);
    const chroma = new Float64Array(capacity);
    const luminance = new Float64Array(capacity);
    const rgb = new Uint8Array(capacity * 3);
    lightness[0] = source_lightness;
    chroma[0] = source_chroma;
    luminance[0] = calculate_relative_luminance(source_rgb);
    rgb.set(source_rgb, 0);
    let length = 1;
    for (let lightness_step = 0; lightness_step <= GRID_RESOLUTION; lightness_step += 1) {
        for (let chroma_step = 0; chroma_step <= chroma_step_count; chroma_step += 1) {
            const point = convert_to_rgb(lightness_step / GRID_RESOLUTION, chroma_step / GRID_RESOLUTION, hue);
            if (!point) continue;
            lightness[length] = lightness_step / GRID_RESOLUTION;
            chroma[length] = chroma_step / GRID_RESOLUTION;
            luminance[length] = calculate_relative_luminance(point);
            rgb.set(point, length * 3);
            length += 1;
        }
    }
    return {
        hue,
        length,
        lightness: lightness.subarray(0, length),
        chroma: chroma.subarray(0, length),
        luminance: luminance.subarray(0, length),
        rgb: rgb.subarray(0, length * 3),
    };
}

function read_candidate(grid, index) {
    return {
        lightness: grid.lightness[index],
        chroma: grid.chroma[index],
        hue: grid.hue,
        rgb: Array.from(grid.rgb.subarray(index * 3, index * 3 + 3)),
        exact: index === 0,
        luminance: grid.luminance[index],
    };
}

function find_nearest_candidate(grid, accepts) {
    let nearest = -1;
    let nearest_distance = Infinity;
    for (let index = 0; index < grid.length; index += 1) {
        const distance = (grid.lightness[index] - grid.lightness[0]) ** 2 + (grid.chroma[index] - grid.chroma[0]) ** 2;
        if (distance < nearest_distance && accepts(grid.luminance[index])) {
            nearest_distance = distance;
            nearest = index;
        }
    }
    return nearest < 0 ? null : read_candidate(grid, nearest);
}

function solve_emphasis(scheme_set, grid) {
    const scheme_list = Object.values(scheme_set).map((scheme) => ({
        text_contrast: scheme.text_contrast,
        paper: calculate_relative_luminance(scheme.paper),
        highlight_paper: calculate_relative_luminance(scheme.highlight_paper),
    }));
    return find_nearest_candidate(grid, (luminance) => scheme_list.every((scheme) => calculate_contrast(luminance, scheme.paper) >= scheme.text_contrast
        && calculate_contrast(luminance, scheme.highlight_paper) >= CONTRAST_GRAPHICAL));
}

function format_color(candidate, source) {
    return candidate.exact
        ? source
        : `oklch(from ${source} ${candidate.lightness.toFixed(3)} ${candidate.chroma.toFixed(3)} h)`;
}

export function solve(hue) {
    const source_rgb = find_vivid_rgb(SOURCE_LIGHTNESS, hue);
    const grid = build_grid(source_rgb);
    const source = `rgb(${source_rgb.join(" ")})`;
    const light = solve_emphasis(FAMILY.light, grid);
    const dark = solve_emphasis(FAMILY.dark, grid);
    if (!light || !dark) return null;
    return `light-dark(${format_color(light, source)}, ${format_color(dark, source)})`;
}

export function apply(root, hue) {
    const result = solve(hue);
    if (!result) return;
    root.style.setProperty("--color-emphasis-ink", result);
}

export function start() {
    const input = document.querySelector('input[name="hue"]');
    return input ? { input } : null;
}

export function bind(state) {
    if (!state) return;
    state.input.addEventListener("change", () => apply(document.documentElement, Number(state.input.value)));
}
