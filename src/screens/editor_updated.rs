use bevy::{
    gizmos::gizmos::Gizmos,
    input::{ButtonState, mouse::MouseButtonInput},
    math::{cubic_splines::*, vec2, curves::sample_derivative::WithDerivative},
    prelude::*
};

// ... (rest of the imports and other code remains the same)

/// This system draws the current [`Curve`] with rectangles placed along the curve,
/// oriented to follow the tangent at each point.
fn draw_curve(curve: Res<Curve>, mut gizmos: Gizmos) {
    let Some(ref curve) = curve.0 else {
        return;
    };
    
    // Draw the curve itself
    let resolution = 100 * curve.segments().len();
    gizmos.linestrip(
        curve.iter_positions(resolution).map(|pt| pt.extend(0.0)),
        Color::srgb(1.0, 1.0, 1.0),
    );
    
    // Draw rectangles along the curve
    let segment_count = 20; // Number of rectangles to draw
    let width = 20.0;      // Width of each rectangle
    
    for i in 0..=segment_count {
        let t = i as f32 / segment_count as f32;
        
        // Get position and derivative (tangent) at this point on the curve
        if let Some(WithDerivative { value: position, derivative }) = curve.sample_with_derivative(t) {
            if derivative.length_squared() > 0.0 {
                // Calculate the normal vector (perpendicular to tangent)
                let tangent = derivative.normalize();
                let normal = Vec2::new(-tangent.y, tangent.x);
                
                // Calculate the four corners of the rectangle
                let half_width = width / 2.0;
                let p1 = position + normal * half_width;
                let p2 = position - normal * half_width;
                let p3 = p2 + tangent * 5.0; // Small depth to make the rectangle visible
                let p4 = p1 + tangent * 5.0;
                
                // Draw the rectangle
                gizmos.linestrip_gradient_2d([
                    (p1, Color::RED),
                    (p2, Color::GREEN),
                    (p3, Color::BLUE),
                    (p4, Color::YELLOW),
                    (p1, Color::RED), // Close the rectangle
                ]);
                
                // Draw the tangent line for debugging
                gizmos.line_2d(position, position + tangent * 30.0, Color::GREEN);
            }
        }
    }
}

// ... (rest of the file remains the same)
