use log::debug;
use voronator::delaunator::Point;

trait Metrics {
    fn distance(&self, other: &Self) -> f64;
}

impl Metrics for Point {
    fn distance(&self, other: &Self) -> f64 {
        let xs = self.x - other.x;
        let ys = self.y - other.y;
        ((xs * xs) + (ys * ys)).sqrt()
    }
}

fn total_distance(tour: &Vec<Point>) -> f64 {
    tour.windows(2)
        .map(|points| points[0].distance(&points[1]))
        .sum()
}

pub fn make_nn_tour(points: Vec<Point>) -> Vec<Point> {
    let mut remaining = points;
    let mut tour = Vec::new();

    if remaining.len() == 0 {
        return tour;
    }

    tour.push(remaining.remove(0));

    while remaining.len() > 0 {
        let mut minimum = f64::MAX;
        let mut index = 0;
        let current = tour.last().unwrap();

        for (pos, point) in remaining.iter().enumerate() {
            let distance = current.distance(point);

            if distance < minimum {
                minimum = distance;
                index = pos;
            }
        }

        tour.push(remaining.remove(index));
    }

    tour
}

fn optimize_two_opt_tour(tour: Vec<Point>) -> (Vec<Point>, f64) {
    let len = tour.len();

    if len == 0 {
        return (tour, 0.0);
    }

    let mut tour = tour;
    let mut prev_distance: f64 = total_distance(&tour);
    let old_distance = prev_distance;

    for i in 1..len - 2 {
        for k in i + 1..len - 1 {
            let new_distance = prev_distance - tour[i - 1].distance(&tour[i])
                + tour[i - 1].distance(&tour[k - 1])
                - tour[k - 1].distance(&tour[k])
                + tour[i].distance(&tour[k]);

            if new_distance < prev_distance {
                prev_distance = new_distance;

                let mut j = 0;

                while i + j < k - j - 1 {
                    let tmp = tour[i + j].clone();
                    tour[i + j] = tour[k - j - 1].clone();
                    tour[k - j - 1] = tmp;
                    j += 1;
                }
            }
        }
    }

    let improvement = (old_distance - prev_distance) / old_distance;
    debug!("Tour improved by {:.3}", improvement);
    (tour, improvement)
}

pub fn optimize(tour: Vec<Point>, criteria: f64) -> Vec<Point> {
    let mut tour = tour;

    loop {
        let (improved_tour, improvement) = optimize_two_opt_tour(tour);
        tour = improved_tour;

        if improvement < criteria {
            return tour;
        }
    }
}
