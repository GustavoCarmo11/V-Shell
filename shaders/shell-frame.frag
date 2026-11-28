// V-Shell unified shell surface with a velocity-driven BlobRect deformation.
// The Dashboard's own widget matrix is mirrored here by inverse-transforming
// the SDF query point. This keeps the panel fill and panel contents in lockstep.
//
// values[0] = rail width, frame thickness, inner-left radius, inner-right radius
// values[1] = shared surface RGBA
// values[2] = Dashboard x, y, width, height (undeformed layout rectangle)
// values[3] = Dashboard radius, open progress, merge radius, reserved
// values[4] = Dashboard affine deformation matrix: m00, m01, m10, m11
// values[5] = Launcher x, y, width, height
// values[6] = Launcher radius, progress, merge radius, reserved
// values[7] = Launcher affine deformation matrix: m00, m01, m10, m11
// values[8] = Status pop-out x, y, width, height
// values[9] = Status pop-out radius, progress, merge radius, reserved
// values[10] = Status pop-out affine deformation matrix: m00, m01, m10, m11

float sdRoundedBoxPerCorner(
    vec2 p,
    vec2 halfSize,
    float topLeftRadius,
    float topRightRadius,
    float bottomRightRadius,
    float bottomLeftRadius
) {
    float radius;
    if (p.x < 0.0) {
        radius = (p.y < 0.0) ? topLeftRadius : bottomLeftRadius;
    } else {
        radius = (p.y < 0.0) ? topRightRadius : bottomRightRadius;
    }

    radius = clamp(radius, 0.0, min(halfSize.x, halfSize.y));
    vec2 q = abs(p) - halfSize + vec2(radius);
    return length(max(q, vec2(0.0)))
        + min(max(q.x, q.y), 0.0)
        - radius;
}

float smoothMin(float a, float b, float radius) {
    float k = max(radius, 0.001);
    float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

void main() {
    vec2 p = uv * size;

    float railWidth = values[0].x;
    float frameThickness = values[0].y;
    float innerLeftRadius = values[0].z;
    float innerRightRadius = values[0].w;

    vec4 surface = values[1];
    vec4 panel = values[2];
    float panelRadius = values[3].x;
    float progress = clamp(values[3].y, 0.0, 1.0);
    float mergeRadius = values[3].z * smoothstep(0.0, 0.0, progress);

    // The frame is the complement of its rounded inner opening. The viewport
    // clips the outside, so all four screen corners remain fully covered.
    vec2 innerMin = vec2(railWidth, frameThickness);
    vec2 innerMax = size - vec2(frameThickness);
    vec2 innerSize = max(innerMax - innerMin, vec2(1.0));
    vec2 innerCenter = innerMin + innerSize * 0.5;

    float innerDistance = sdRoundedBoxPerCorner(
        p - innerCenter,
        innerSize * 0.5,
        innerLeftRadius,
        innerRightRadius,
        innerRightRadius,
        innerLeftRadius
    );
    float frameDistance = -innerDistance;
    float shapeDistance = frameDistance;

    if (progress > 0.001) {
        vec2 panelSize = panel.zw;
        vec2 panelCenter = panel.xy + panelSize * 0.5;
        vec2 screenDelta = p - panelCenter;

        // Invert the same centered matrix applied to the Dashboard Rectangle.
        // The V-Shell deformer is symmetric: [m00 m01; m01 m11].
        float m00 = values[4].x;
        float m01 = values[4].y;
        float m10 = values[4].z;
        float m11 = values[4].w;
        float determinant = m00 * m11 - m01 * m10;
        determinant = abs(determinant) < 0.0001 ? 1.0 : determinant;

        vec2 panelLocal = vec2(
            (m11 * screenDelta.x - m01 * screenDelta.y) / determinant,
            (-m10 * screenDelta.x + m00 * screenDelta.y) / determinant
        );

        float panelDistance = sdRoundedBoxPerCorner(
            panelLocal,
            panelSize * 0.5,
            0.0,
            0.0,
            panelRadius,
            panelRadius
        );

        shapeDistance = smoothMin(frameDistance, panelDistance, mergeRadius);
    }

    // The Launcher joins the same surface at the bottom perimeter. Its lower
    // edge is aligned with the inner edge of the bottom frame; square lower
    // corners allow the SDF union to form a continuous bridge back to the rail.
    vec4 launcher = values[5];
    float launcherProgress = clamp(values[6].y, 0.0, 1.0);
    if (launcherProgress > 0.001) {
        vec2 launcherSize = launcher.zw;
        vec2 launcherCenter = launcher.xy + launcherSize * 0.5;
        vec2 launcherDelta = p - launcherCenter;

        float lm00 = values[7].x;
        float lm01 = values[7].y;
        float lm10 = values[7].z;
        float lm11 = values[7].w;
        float launcherDeterminant = lm00 * lm11 - lm01 * lm10;
        launcherDeterminant = abs(launcherDeterminant) < 0.0001
            ? 1.0
            : launcherDeterminant;

        vec2 launcherLocal = vec2(
            (lm11 * launcherDelta.x - lm01 * launcherDelta.y) / launcherDeterminant,
            (-lm10 * launcherDelta.x + lm00 * launcherDelta.y) / launcherDeterminant
        );

        float launcherRadius = values[6].x;
        float launcherDistance = sdRoundedBoxPerCorner(
            launcherLocal,
            launcherSize * 0.5,
            launcherRadius,
            launcherRadius,
            0.0,
            0.0
        );

        float launcherMerge = values[6].z * smoothstep(0.0, 0.0, launcherProgress);
        shapeDistance = smoothMin(shapeDistance, launcherDistance, launcherMerge);
    }

    // Status pop-outs share one surface with the rail. Their left corners are
    // square so the morphing SDF can join the panel to the bar without a seam.
    vec4 statusPanel = values[8];
    vec4 statusStyle = values[9];
    float statusProgress = clamp(statusStyle.y, 0.0, 1.0);
    if (statusProgress > 0.001) {
        vec2 statusSize = statusPanel.zw;
        vec2 statusCenter = statusPanel.xy + statusSize * 0.5;
        vec2 statusDelta = p - statusCenter;

        float sm00 = values[10].x;
        float sm01 = values[10].y;
        float sm10 = values[10].z;
        float sm11 = values[10].w;
        float statusDeterminant = sm00 * sm11 - sm01 * sm10;
        statusDeterminant = abs(statusDeterminant) < 0.0001
            ? 1.0
            : statusDeterminant;

        vec2 statusLocal = vec2(
            (sm11 * statusDelta.x - sm01 * statusDelta.y) / statusDeterminant,
            (-sm10 * statusDelta.x + sm00 * statusDelta.y) / statusDeterminant
        );

        float statusRadius = statusStyle.x;
        float statusDistance = sdRoundedBoxPerCorner(
            statusLocal,
            statusSize * 0.5,
            0.0,
            statusRadius,
            0.0,
            0.0
        );
        shapeDistance = smoothMin(shapeDistance, statusDistance, statusStyle.z);
    }

    float aa = max(fwidth(shapeDistance), 0.65);
    float coverage = 1.0 - smoothstep(-aa, aa, shapeDistance);
    color = vec4(surface.rgb, surface.a * coverage);
}
