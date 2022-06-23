use crate::constants;
use crate::to_i32::to_i32;
use crate::StrError;
use plotpy::{Canvas, Plot, PolyCode};
use std::collections::HashMap;

#[repr(C)]
pub(crate) struct ExtTriangle {
    data: [u8; 0],
    marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

extern "C" {
    // Triangle
    fn new_triangle(npoint: i32, nsegment: i32, nregion: i32, nhole: i32) -> *mut ExtTriangle;
    fn drop_triangle(triangle: *mut ExtTriangle);
    fn set_point(triangle: *mut ExtTriangle, index: i32, x: f64, y: f64) -> i32;
    fn set_segment(triangle: *mut ExtTriangle, index: i32, a: i32, b: i32) -> i32;
    fn set_region(
        triangle: *mut ExtTriangle,
        index: i32,
        x: f64,
        y: f64,
        attribute: i32,
        max_area: f64,
    ) -> i32;
    fn set_hole(triangle: *mut ExtTriangle, index: i32, x: f64, y: f64) -> i32;
    fn run_delaunay(triangle: *mut ExtTriangle, verbose: i32) -> i32;
    fn run_voronoi(triangle: *mut ExtTriangle, verbose: i32) -> i32;
    fn run_triangulate(
        triangle: *mut ExtTriangle,
        verbose: i32,
        quadratic: i32,
        global_max_area: f64,
        global_min_angle: f64,
    ) -> i32;
    fn get_npoint(triangle: *mut ExtTriangle) -> i32;
    fn get_ntriangle(triangle: *mut ExtTriangle) -> i32;
    fn get_ncorner(triangle: *mut ExtTriangle) -> i32;
    fn get_point(triangle: *mut ExtTriangle, index: i32, dim: i32) -> f64;
    fn get_triangle_corner(triangle: *mut ExtTriangle, index: i32, corner: i32) -> i32;
    fn get_triangle_attribute(triangle: *mut ExtTriangle, index: i32) -> i32;
    fn get_voronoi_npoint(triangle: *mut ExtTriangle) -> i32;
    fn get_voronoi_point(triangle: *mut ExtTriangle, index: i32, dim: i32) -> f64;
    fn get_voronoi_nedge(triangle: *mut ExtTriangle) -> i32;
    fn get_voronoi_edge_point(triangle: *mut ExtTriangle, index: i32, side: i32) -> i32;
    fn get_voronoi_edge_point_b_direction(triangle: *mut ExtTriangle, index: i32, dim: i32) -> f64;
}

/// Holds the index of an endpoint on a Voronoi edge or the direction of the Voronoi edge
#[derive(Clone, Debug)]
pub enum VoronoiEdgePoint {
    /// The index of the endpoint
    Index(usize),

    /// The direction of the infinite ray
    Direction(f64, f64),
}

/// Maps indices used in this library (tritet) to indices used in Triangle
///
/// ```text
/// This library (tritet)      Triangle
///         NODES               CORNERS
///           2                    2
///          / \                  / \
///         /   \                /   \
///        5     4              4     3
///       /       \            /       \
///      /         \          /         \
///     0-----3-----1        0-----5-----1
/// ```
const TRITET_TO_TRIANGLE: [usize; 6] = [0, 1, 2, 5, 3, 4];

/// Defines a set of "light" colors
const LIGHT_COLORS: [&'static str; 17] = [
    "#cbe4f9", "#cdf5f6", "#eff9da", "#f9ebdf", "#f9d8d6", "#d6cdea", "#acddde", "#caf1de",
    "#e1f8dc", "#fef8dd", "#ffe7c7", "#f7d8ba", "#d0fffe", "#fffddb", "#e4ffde", "#ffd3fd",
    "#ffe7d3",
];

/// Implements high-level functions to call Shewchuk's Triangle C-Code
pub struct Triangle {
    ext_triangle: *mut ExtTriangle, // data allocated by the c-code
    npoint: usize,                  // number of points
    nsegment: Option<usize>,        // number of segments
    nregion: Option<usize>,         // number of regions
    nhole: Option<usize>,           // number of holes
    all_points_set: bool,           // indicates that all points have been set
    all_segments_set: bool,         // indicates that all segments have been set
    all_regions_set: bool,          // indicates that all regions have been set
    all_holes_set: bool,            // indicates that all holes have been set
}

impl Triangle {
    /// Allocates a new instance
    pub fn new(
        npoint: usize,
        nsegment: Option<usize>,
        nregion: Option<usize>,
        nhole: Option<usize>,
    ) -> Result<Self, StrError> {
        if npoint < 3 {
            return Err("npoint must be ≥ 3");
        }
        let npoint_i32: i32 = to_i32(npoint);
        let nsegment_i32: i32 = match nsegment {
            Some(v) => to_i32(v),
            None => 0,
        };
        let nregion_i32: i32 = match nregion {
            Some(v) => to_i32(v),
            None => 0,
        };
        let nhole_i32: i32 = match nhole {
            Some(v) => to_i32(v),
            None => 0,
        };
        unsafe {
            let ext_triangle = new_triangle(npoint_i32, nsegment_i32, nregion_i32, nhole_i32);
            if ext_triangle.is_null() {
                return Err("INTERNAL ERROR: Cannot allocate ExtTriangle");
            }
            Ok(Triangle {
                ext_triangle,
                npoint,
                nsegment,
                nregion,
                nhole,
                all_points_set: false,
                all_segments_set: false,
                all_regions_set: false,
                all_holes_set: false,
            })
        }
    }

    /// Sets the point coordinates
    pub fn set_point(&mut self, index: usize, x: f64, y: f64) -> Result<&mut Self, StrError> {
        unsafe {
            let status = set_point(self.ext_triangle, to_i32(index), x, y);
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_POINT_LIST {
                    return Err("INTERNAL ERROR: Found NULL point list");
                }
                if status == constants::TRITET_ERROR_INVALID_POINT_INDEX {
                    return Err("Index of point is out of bounds");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        if index == self.npoint - 1 {
            self.all_points_set = true;
        } else {
            self.all_points_set = false;
        }
        Ok(self)
    }

    /// Sets the segment endpoint IDs
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the segment and goes from 0 to `nsegment` (passed down to `new`)
    /// * `a` -- is the ID (index) of the first point on the segment
    /// * `b` -- is the ID (index) of the second point on the segment
    pub fn set_segment(&mut self, index: usize, a: usize, b: usize) -> Result<&mut Self, StrError> {
        let nsegment = match self.nsegment {
            Some(n) => n,
            None => {
                return Err(
                    "The number of segments (given to 'new') must not be None to set segment",
                )
            }
        };
        unsafe {
            let status = set_segment(self.ext_triangle, to_i32(index), to_i32(a), to_i32(b));
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_SEGMENT_LIST {
                    return Err("INTERNAL ERROR: Found NULL segment list");
                }
                if status == constants::TRITET_ERROR_INVALID_SEGMENT_INDEX {
                    return Err("Index of segment is out of bounds");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        if index == nsegment - 1 {
            self.all_segments_set = true;
        } else {
            self.all_segments_set = false;
        }
        Ok(self)
    }

    /// Marks a region within the Planar Straight Line Graph (PSLG)
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the region and goes from 0 to `nregion` (passed down to `new`)
    /// * `x` -- is the x-coordinate of the hole
    /// * `y` -- is the x-coordinate of the hole
    /// * `attribute` -- is the attribute ID to group the triangles belonging to this region
    /// * `max_area` -- is the maximum area constraint for the triangles belonging to this region
    pub fn set_region(
        &mut self,
        index: usize,
        x: f64,
        y: f64,
        attribute: usize,
        max_area: Option<f64>,
    ) -> Result<&mut Self, StrError> {
        let nregion = match self.nregion {
            Some(n) => n,
            None => {
                return Err("The number of regions (given to 'new') must not be None to set region")
            }
        };
        let area_constraint = match max_area {
            Some(v) => v,
            None => -1.0,
        };
        unsafe {
            let status = set_region(
                self.ext_triangle,
                to_i32(index),
                x,
                y,
                to_i32(attribute),
                area_constraint,
            );
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_REGION_LIST {
                    return Err("INTERNAL ERROR: Found NULL region list");
                }
                if status == constants::TRITET_ERROR_INVALID_REGION_INDEX {
                    return Err("Index of region is out of bounds");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        if index == nregion - 1 {
            self.all_regions_set = true;
        } else {
            self.all_regions_set = false;
        }
        Ok(self)
    }

    /// Marks a hole within the Planar Straight Line Graph (PSLG)
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the hole and goes from 0 to `nhole` (passed down to `new`)
    /// * `x` -- is the x-coordinate of the hole
    /// * `y` -- is the x-coordinate of the hole
    pub fn set_hole(&mut self, index: usize, x: f64, y: f64) -> Result<&mut Self, StrError> {
        let nhole = match self.nhole {
            Some(n) => n,
            None => {
                return Err("The number of holes (given to 'new') must not be None to set hole")
            }
        };
        unsafe {
            let status = set_hole(self.ext_triangle, to_i32(index), x, y);
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_HOLE_LIST {
                    return Err("INTERNAL ERROR: Found NULL hole list");
                }
                if status == constants::TRITET_ERROR_INVALID_HOLE_INDEX {
                    return Err("Index of hole is out of bounds");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        if index == nhole - 1 {
            self.all_holes_set = true;
        } else {
            self.all_holes_set = false;
        }
        Ok(self)
    }

    /// Generates a Delaunay triangulation
    ///
    /// # Input
    ///
    /// * `verbose` -- Prints Triangle's messages to the console
    pub fn generate_delaunay(&self, verbose: bool) -> Result<(), StrError> {
        if !self.all_points_set {
            return Err("All points must be set to generate Delaunay triangulation");
        }
        unsafe {
            let status = run_delaunay(self.ext_triangle, if verbose { 1 } else { 0 });
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_POINT_LIST {
                    return Err("INTERNAL ERROR: Found NULL point list");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        Ok(())
    }

    /// Generates a Voronoi tessellation and Delaunay triangulation
    ///
    /// # Input
    ///
    /// * `verbose` -- Prints Triangle's messages to the console
    pub fn generate_voronoi(&self, verbose: bool) -> Result<(), StrError> {
        if !self.all_points_set {
            return Err("All points must be set to generate Voronoi tessellation");
        }
        unsafe {
            let status = run_voronoi(self.ext_triangle, if verbose { 1 } else { 0 });
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_POINT_LIST {
                    return Err("INTERNAL ERROR: Found NULL point list");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        Ok(())
    }

    /// Generates a conforming constrained Delaunay triangulation with some quality constraints
    ///
    /// # Input
    ///
    /// * `verbose` -- Prints Triangle's messages to the console
    /// * `quadratic` -- Generates the middle nodes; e.g., nnode = 6
    /// * `global_max_area` -- The maximum area constraint for all generated triangles
    /// * `global_min_angle` -- The minimum angle constraint is given in degrees (the default minimum angle is twenty degrees)
    pub fn generate_mesh(
        &mut self,
        verbose: bool,
        quadratic: bool,
        global_max_area: Option<f64>,
        global_min_angle: Option<f64>,
    ) -> Result<(), StrError> {
        if !self.all_points_set {
            return Err("All points must be set to generate mesh");
        }
        if !self.all_segments_set {
            return Err("All segments must be set to generate mesh");
        }
        let max_area = match global_max_area {
            Some(v) => v,
            None => 0.0,
        };
        let min_angle = match global_min_angle {
            Some(v) => v,
            None => 0.0,
        };
        unsafe {
            let status = run_triangulate(
                self.ext_triangle,
                if verbose { 1 } else { 0 },
                if quadratic { 1 } else { 0 },
                max_area,
                min_angle,
            );
            if status != constants::TRITET_SUCCESS {
                if status == constants::TRITET_ERROR_NULL_DATA {
                    return Err("INTERNAL ERROR: Found NULL data");
                }
                if status == constants::TRITET_ERROR_NULL_POINT_LIST {
                    return Err("INTERNAL ERROR: Found NULL point list");
                }
                if status == constants::TRITET_ERROR_NULL_SEGMENT_LIST {
                    return Err("List of segments must be defined first");
                }
                if status == constants::TRITET_ERROR_STRING_CONCAT {
                    return Err("Cannot write string with commands for Triangle");
                }
                return Err("INTERNAL ERROR: Some error occurred");
            }
        }
        Ok(())
    }

    /// Returns the number of points of the Delaunay triangulation (constrained or not)
    pub fn npoint(&self) -> usize {
        unsafe { get_npoint(self.ext_triangle) as usize }
    }

    /// Returns the number of triangles on the Delaunay triangulation (constrained or not)
    pub fn ntriangle(&self) -> usize {
        unsafe { get_ntriangle(self.ext_triangle) as usize }
    }

    /// Returns the number of nodes on a triangle (e.g., 3 or 6)
    ///
    /// ```text
    ///     NODES
    ///       2
    ///      / \     The middle nodes are
    ///     /   \    only generated if the
    ///    5     4   quadratic flag is true
    ///   /       \
    ///  /         \
    /// 0-----3-----1
    /// ```
    pub fn nnode(&self) -> usize {
        unsafe { get_ncorner(self.ext_triangle) as usize }
    }

    /// Returns the x-y coordinates of a point
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the point and goes from 0 to `npoint`
    /// * `dim` -- is the space dimension index: 0 or 1
    ///
    /// # Warning
    ///
    /// This function will return 0.0 if either `index` or `dim` are out of range.
    pub fn point(&self, index: usize, dim: usize) -> f64 {
        unsafe { get_point(self.ext_triangle, to_i32(index), to_i32(dim)) }
    }

    /// Returns the ID of a Triangle's node
    ///
    /// ```text
    ///     NODES
    ///       2
    ///      / \     The middle nodes are
    ///     /   \    only generated if the
    ///    5     4   quadratic flag is true
    ///   /       \
    ///  /         \
    /// 0-----3-----1
    /// ```
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the triangle and goes from 0 to `ntriangle`
    /// * `m` -- is the local index of the node and goes from 0 to `nnode`
    ///
    /// # Warning
    ///
    /// This function will return 0 if either `index` or `m` are out of range.
    pub fn triangle_node(&self, index: usize, m: usize) -> usize {
        unsafe {
            let corner = TRITET_TO_TRIANGLE[m];
            get_triangle_corner(self.ext_triangle, to_i32(index), to_i32(corner)) as usize
        }
    }

    /// Returns the attribute ID of a triangle
    ///
    /// # Warning
    ///
    /// This function will return 0 if either `index` or `m` are out of range.
    pub fn triangle_attribute(&self, index: usize) -> usize {
        unsafe { get_triangle_attribute(self.ext_triangle, to_i32(index)) as usize }
    }

    /// Returns the number of points of the Voronoi tessellation
    pub fn voronoi_npoint(&self) -> usize {
        unsafe { get_voronoi_npoint(self.ext_triangle) as usize }
    }

    /// Returns the x-y coordinates of a point on the Voronoi tessellation
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the point and goes from 0 to `voronoi_npoint`
    /// * `dim` -- is the space dimension index: 0 or 1
    ///
    /// # Warning
    ///
    /// This function will return 0.0 if either `index` or `dim` are out of range.
    pub fn voronoi_point(&self, index: usize, dim: usize) -> f64 {
        unsafe { get_voronoi_point(self.ext_triangle, to_i32(index), to_i32(dim)) }
    }

    /// Returns the number of edges on the Voronoi tessellation
    pub fn voronoi_nedge(&self) -> usize {
        unsafe { get_voronoi_nedge(self.ext_triangle) as usize }
    }

    /// Returns the index of an endpoint on a Voronoi edge or the direction of the Voronoi edge
    ///
    /// # Input
    ///
    /// * `index` -- is the index of the edge and goes from 0 to `voronoi_nedge`
    /// * `side` -- indicates the endpoint: 0 or 1
    ///
    /// # Warning
    ///
    /// This function will return Index(0) if either `index` or `side` are out of range.
    pub fn voronoi_edge_point(&self, index: usize, side: usize) -> VoronoiEdgePoint {
        unsafe {
            let index_i32 = to_i32(index);
            let id = get_voronoi_edge_point(self.ext_triangle, index_i32, to_i32(side));
            if id == -1 {
                let x = get_voronoi_edge_point_b_direction(self.ext_triangle, index_i32, 0);
                let y = get_voronoi_edge_point_b_direction(self.ext_triangle, index_i32, 1);
                VoronoiEdgePoint::Direction(x, y)
            } else {
                VoronoiEdgePoint::Index(id as usize)
            }
        }
    }

    /// Draw triangles
    pub fn draw_triangles(&self) -> Plot {
        let mut plot = Plot::new();
        let n_triangle = self.ntriangle();
        if n_triangle < 1 {
            return plot;
        }
        let mut canvas = Canvas::new();
        canvas.set_edge_color("black");
        let mut x = vec![0.0; 2];
        let mut min = vec![f64::MAX; 2];
        let mut max = vec![f64::MIN; 2];
        let mut colors: HashMap<usize, &'static str> = HashMap::new();
        let mut index_color = 0;
        for tri in 0..n_triangle {
            let attribute = self.triangle_attribute(tri);
            let color = match colors.get(&attribute) {
                Some(c) => c,
                None => {
                    let c = LIGHT_COLORS[index_color % LIGHT_COLORS.len()];
                    colors.insert(attribute, c);
                    index_color += 1;
                    c
                }
            };
            canvas.set_face_color(color);
            canvas.polycurve_begin();
            for m in 0..3 {
                let p = self.triangle_node(tri, m);
                for dim in 0..2 {
                    x[dim] = self.point(p, dim);
                    min[dim] = f64::min(min[dim], x[dim]);
                    max[dim] = f64::max(max[dim], x[dim]);
                }
                if m == 0 {
                    canvas.polycurve_add(x[0], x[1], PolyCode::MoveTo);
                } else {
                    canvas.polycurve_add(x[0], x[1], PolyCode::LineTo);
                }
            }
            canvas.polycurve_end(true);
        }
        plot.set_range(min[0], max[0], min[1], max[1]).add(&canvas);
        plot
    }
}

impl Drop for Triangle {
    /// Tells the c-code to release memory
    fn drop(&mut self) {
        unsafe {
            drop_triangle(self.ext_triangle);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::Triangle;
    use crate::{StrError, VoronoiEdgePoint};

    #[test]
    fn derive_works() {
        let option = VoronoiEdgePoint::Index(0);
        let cloned = option.clone();
        assert_eq!(format!("{:?}", option), "Index(0)");
        assert_eq!(format!("{:?}", cloned), "Index(0)");
    }

    #[test]
    fn new_captures_some_errors() {
        assert_eq!(
            Triangle::new(2, None, None, None).err(),
            Some("npoint must be ≥ 3")
        );
    }

    #[test]
    fn new_works() -> Result<(), StrError> {
        let triangle = Triangle::new(3, Some(3), None, None)?;
        assert_eq!(triangle.ext_triangle.is_null(), false);
        assert_eq!(triangle.npoint, 3);
        assert_eq!(triangle.nsegment, Some(3));
        assert_eq!(triangle.nregion, None);
        assert_eq!(triangle.nhole, None);
        assert_eq!(triangle.all_points_set, false);
        assert_eq!(triangle.all_segments_set, false);
        assert_eq!(triangle.all_regions_set, false);
        assert_eq!(triangle.all_holes_set, false);
        Ok(())
    }

    #[test]
    fn set_point_captures_some_errors() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        assert_eq!(
            triangle.set_point(4, 0.0, 0.0).err(),
            Some("Index of point is out of bounds")
        );
        Ok(())
    }

    #[test]
    fn set_segment_captures_some_errors() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        assert_eq!(
            triangle.set_segment(0, 0, 1).err(),
            Some("The number of segments (given to 'new') must not be None to set segment")
        );
        let mut triangle = Triangle::new(3, Some(3), None, None)?;
        assert_eq!(
            triangle.set_segment(4, 0, 1).err(),
            Some("Index of segment is out of bounds")
        );
        Ok(())
    }

    #[test]
    fn set_region_captures_some_errors() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        assert_eq!(
            triangle.set_region(0, 0.33, 0.33, 1, Some(0.1)).err(),
            Some("The number of regions (given to 'new') must not be None to set region")
        );
        let mut triangle = Triangle::new(3, Some(3), Some(1), None)?;
        assert_eq!(
            triangle.set_region(1, 0.33, 0.33, 1, Some(0.1)).err(),
            Some("Index of region is out of bounds")
        );
        Ok(())
    }

    #[test]
    fn set_hole_captures_some_errors() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        assert_eq!(
            triangle.set_hole(0, 0.33, 0.33).err(),
            Some("The number of holes (given to 'new') must not be None to set hole")
        );
        let mut triangle = Triangle::new(3, Some(3), Some(1), Some(1))?;
        assert_eq!(
            triangle.set_hole(1, 0.33, 0.33).err(),
            Some("Index of hole is out of bounds")
        );
        Ok(())
    }

    #[test]
    fn delaunay_1_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?;
        triangle.generate_delaunay(false)?;
        assert_eq!(triangle.npoint(), 3);
        assert_eq!(triangle.ntriangle(), 1);
        assert_eq!(triangle.nnode(), 3);
        assert_eq!(triangle.point(0, 0), 0.0);
        assert_eq!(triangle.point(0, 1), 0.0);
        assert_eq!(triangle.point(1, 0), 1.0);
        assert_eq!(triangle.point(1, 1), 0.0);
        assert_eq!(triangle.point(2, 0), 0.0);
        assert_eq!(triangle.point(2, 1), 1.0);
        assert_eq!(triangle.triangle_node(0, 0), 0);
        assert_eq!(triangle.triangle_node(0, 1), 1);
        assert_eq!(triangle.triangle_node(0, 2), 2);
        assert_eq!(triangle.voronoi_npoint(), 0);
        assert_eq!(triangle.voronoi_nedge(), 0);
        Ok(())
    }

    #[test]
    fn voronoi_1_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, None, None, None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?;
        triangle.generate_voronoi(false)?;
        assert_eq!(triangle.npoint(), 3);
        assert_eq!(triangle.ntriangle(), 1);
        assert_eq!(triangle.nnode(), 3);
        assert_eq!(triangle.point(0, 0), 0.0);
        assert_eq!(triangle.point(0, 1), 0.0);
        assert_eq!(triangle.point(1, 0), 1.0);
        assert_eq!(triangle.point(1, 1), 0.0);
        assert_eq!(triangle.point(2, 0), 0.0);
        assert_eq!(triangle.point(2, 1), 1.0);
        assert_eq!(triangle.triangle_node(0, 0), 0);
        assert_eq!(triangle.triangle_node(0, 1), 1);
        assert_eq!(triangle.triangle_node(0, 2), 2);
        assert_eq!(triangle.voronoi_npoint(), 1);
        assert_eq!(triangle.voronoi_point(0, 0), 0.5);
        assert_eq!(triangle.voronoi_point(0, 1), 0.5);
        assert_eq!(triangle.voronoi_nedge(), 3);
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(0, 0)),
            "Index(0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(0, 1)),
            "Direction(0.0, -1.0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(1, 0)),
            "Index(0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(1, 1)),
            "Direction(1.0, 1.0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(2, 0)),
            "Index(0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(2, 1)),
            "Direction(-1.0, 0.0)"
        );
        Ok(())
    }

    #[test]
    fn mesh_1_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, Some(3), None, None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?;
        triangle
            .set_segment(0, 0, 1)?
            .set_segment(1, 1, 2)?
            .set_segment(2, 2, 0)?;
        triangle.generate_mesh(false, false, None, None)?;
        assert_eq!(triangle.npoint(), 3);
        assert_eq!(triangle.ntriangle(), 1);
        assert_eq!(triangle.nnode(), 3);
        assert_eq!(triangle.point(0, 0), 0.0);
        assert_eq!(triangle.point(0, 1), 0.0);
        assert_eq!(triangle.point(1, 0), 1.0);
        assert_eq!(triangle.point(1, 1), 0.0);
        assert_eq!(triangle.point(2, 0), 0.0);
        assert_eq!(triangle.point(2, 1), 1.0);
        assert_eq!(triangle.triangle_node(0, 0), 0);
        assert_eq!(triangle.triangle_node(0, 1), 1);
        assert_eq!(triangle.triangle_node(0, 2), 2);
        assert_eq!(triangle.triangle_attribute(0), 0);
        assert_eq!(triangle.triangle_attribute(1), 0);
        assert_eq!(triangle.triangle_attribute(2), 0);
        assert_eq!(triangle.voronoi_npoint(), 0);
        assert_eq!(triangle.voronoi_nedge(), 0);
        Ok(())
    }

    #[test]
    fn mesh_2_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, Some(3), None, None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?;
        triangle
            .set_segment(0, 0, 1)?
            .set_segment(1, 1, 2)?
            .set_segment(2, 2, 0)?;
        triangle.generate_mesh(false, true, Some(0.1), Some(20.0))?;
        assert_eq!(triangle.npoint(), 22);
        assert_eq!(triangle.ntriangle(), 7);
        assert_eq!(triangle.nnode(), 6);
        Ok(())
    }

    #[test]
    fn get_methods_work_with_wrong_indices() -> Result<(), StrError> {
        let triangle = Triangle::new(3, None, None, None)?;
        assert_eq!(triangle.point(100, 0), 0.0);
        assert_eq!(triangle.point(0, 100), 0.0);
        assert_eq!(triangle.triangle_attribute(100), 0);
        assert_eq!(triangle.voronoi_point(100, 0), 0.0);
        assert_eq!(triangle.voronoi_point(0, 100), 0.0);
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(100, 0)),
            "Index(0)"
        );
        assert_eq!(
            format!("{:?}", triangle.voronoi_edge_point(0, 100)),
            "Index(0)"
        );
        Ok(())
    }

    #[test]
    fn draw_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(3, Some(3), None, None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?;
        triangle
            .set_segment(0, 0, 1)?
            .set_segment(1, 1, 2)?
            .set_segment(2, 2, 0)?;
        triangle.generate_mesh(false, true, Some(0.25), None)?;
        let mut plot = triangle.draw_triangles();
        if false {
            plot.set_equal_axes(true)
                .set_figure_size_points(600.0, 600.0)
                .save("/tmp/tritet/draw_works.svg")?;
        }
        Ok(())
    }

    #[test]
    fn mesh_3_works() -> Result<(), StrError> {
        let mut triangle = Triangle::new(4, Some(3), Some(1), None)?;
        triangle
            .set_point(0, 0.0, 0.0)?
            .set_point(1, 1.0, 0.0)?
            .set_point(2, 0.0, 1.0)?
            .set_point(3, 0.5, 0.5)?
            .set_region(0, 0.5, 0.2, 1, None)?;
        triangle
            .set_segment(0, 0, 1)?
            .set_segment(1, 1, 2)?
            .set_segment(2, 2, 0)?;
        triangle.generate_mesh(false, true, Some(0.25), None)?;
        assert_eq!(triangle.ntriangle(), 2);
        assert_eq!(triangle.triangle_attribute(0), 1);
        assert_eq!(triangle.triangle_attribute(1), 0);
        let mut plot = triangle.draw_triangles();
        if false {
            plot.set_equal_axes(true)
                .set_figure_size_points(600.0, 600.0)
                .save("/tmp/tritet/mesh_3_works.svg")?;
        }
        Ok(())
    }
}
