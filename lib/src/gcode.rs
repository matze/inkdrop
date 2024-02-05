use crate::point::Point;
use askama::Template;
use serde::Deserialize;

pub type Channel = Vec<Point>;
pub type Channels = Vec<Channel>;

#[derive(Template)]
#[template(path = "template.gcode", escape = "none")]
struct GcodeTemplate<'a> {
    calibration: &'a Calibration,
    channel: &'a Channel,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct Calibration {
    /// base width of the machine, i.e., the distance
    /// between the shafts of the stepper
    /// motors
    pub base_width: f64,

    /// vertical distance between the center of the drawing
    /// plane and the shafts of the stepper motors
    pub base_height: f64,

    /// maximal with of the drawing plane, gcode will
    /// be scaled to fill the area, you can set this to
    /// a lower value to have smaller result images. Ratio
    /// will be peserved.
    pub drawing_width: f64,

    /// maximal height of the drawing plane, gcode will
    /// be scaled to fill the area, you can set this to
    /// a lower value to have smaller result images. Ratio
    /// will be preserved.
    pub drawing_height: f64,
}

impl Calibration {
    pub fn gcode(&self, channel: &Channel) -> String {
        let tpl = GcodeTemplate {
            channel,
            calibration: self,
        };
        tpl.render().unwrap()
    }

    pub fn translate_origin(&self, channels: &Channels) -> Channels {
        let mut pts = Vec::new();
        for c in channels {
            pts.append(&mut c.clone());
        }

        let min_x = pts
            .iter()
            .map(|p| p.x)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let min_y = pts
            .iter()
            .map(|p| p.y)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_x = pts
            .iter()
            .map(|p| p.x)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_y = pts
            .iter()
            .map(|p| p.y)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let dx = max_x - min_x;
        let dy = max_y - min_y;

        let ratio_x = self.drawing_width / dx;

        let ratio_y = self.drawing_height / dy;
        let ratio = ratio_x.min(ratio_y);

        // equation: x_max + offset == dx/2
        let offset_x = 0.5 * dx - max_x;
        let offset_y = 0.5 * dy - max_y;

        let mut result = Vec::new();

        for c in channels {
            let mut transformed_channel = Vec::with_capacity(c.len());
            for pt in c {
                transformed_channel.push(Point::new(
                    (pt.x + offset_x) * ratio,
                    (pt.y + offset_y) * ratio,
                ));
            }
            result.push(transformed_channel);
        }

        result
    }

    pub fn apply(&self, pt: &Point) -> Point {
        let pt = Point::new(pt.x, -pt.y);
        let a =
            ((0.5 * self.base_width + pt.x).powf(2.0) + (self.base_height - pt.y).powf(2.0)).sqrt();
        let b =
            ((0.5 * self.base_width - pt.x).powf(2.0) + (self.base_height - pt.y).powf(2.0)).sqrt();

        Point::new(a, b)
    }

    fn transform_single_channel(&self, channel: &Channel) -> Channel {
        let home = self.apply(&Point::origin());
        channel
            .iter()
            .map(|pt| self.apply(pt))
            .map(|pt| Point::new(pt.x - home.x, pt.y - home.y))
            .collect()
    }

    pub fn transform_coordinates(&self, channels: &Channels) -> Channels {
        channels
            .iter()
            .map(|c| self.transform_single_channel(c))
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn conversion_of_coordinates_works() {
        let calib = Calibration {
            base_width: 80.1,
            base_height: 57.3,
            drawing_width: 50.,
            drawing_height: 50.,
            l0_x: 0.,
            l0_y: 0.,
        };

        assert_eq!(
            calib.apply(&Point::origin()),
            Point::new(64.06248902, 64.06248902)
        );
    }

    #[test]
    fn translation_works() {
        let calib = Calibration {
            base_width: 10.,
            base_height: 20.,
            drawing_width: 50.,
            drawing_height: 100.,
            l0_x: 0.,
            l0_y: 0.,
        };

        let channels = vec![vec![Point::new(-1., 15.), Point::new(4., -5.)]];

        let result = calib.translate_origin(&channels);
        assert_eq!(
            result,
            vec![vec![Point { x: -12.5, y: 50. }, Point { x: 12.5, y: -50. }]]
        );

        assert_eq!(result, calib.translate_origin(&result));
    }
}
