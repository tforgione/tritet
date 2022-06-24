use std::thread;
// use std::time::Duration;
use tritet::{StrError, Tetgen};

fn main() {
    println!("Running Mem Check on Tetgen\n");
    let handle_1 = thread::spawn(|| {
        // thread::sleep(Duration::from_millis(750));
        println!("..1..");
        run_all().unwrap();
    });
    let handle_2 = thread::spawn(|| {
        // thread::sleep(Duration::from_millis(500));
        println!("..2..");
        run_all().unwrap();
    });
    let handle_3 = thread::spawn(|| {
        // thread::sleep(Duration::from_millis(250));
        println!("..3..");
        run_all().unwrap();
    });
    handle_1.join().unwrap();
    handle_2.join().unwrap();
    handle_3.join().unwrap();
    println!("\nDone\n");
}

fn run_all() -> Result<(), StrError> {
    println!("run tests on tetgen");
    let _tet = Tetgen::new(4, Some(4), Some(1), Some(1))?;
    new_captures_some_errors();
    set_point_captures_some_errors()?;
    set_facet_npoint_captures_some_errors()?;
    set_facet_point_captures_some_errors()?;
    set_region_captures_some_errors()?;
    set_hole_captures_some_errors()?;
    generate_methods_capture_some_errors()?;
    Ok(())
}

fn new_captures_some_errors() {
    assert_eq!(
        Tetgen::new(3, None, None, None).err(),
        Some("npoint must be ≥ 4")
    );
    assert_eq!(
        Tetgen::new(4, Some(3), None, None).err(),
        Some("nfacet must be ≥ 4")
    );
}

fn set_point_captures_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, None, None, None)?;
    assert_eq!(
        tetgen.set_point(5, 0.0, 0.0, 0.0).err(),
        Some("index of point is out of bounds")
    );
    Ok(())
}

fn set_facet_npoint_captures_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, None, None, None)?;
    assert_eq!(
        tetgen.set_facet_npoint(0, 3).err(),
        Some("cannot set facet npoint because the number of facets is None")
    );
    let mut tetgen = Tetgen::new(4, Some(4), None, None)?;
    assert_eq!(
        tetgen.set_facet_npoint(5, 2).err(),
        Some("npoint on a facet must be ≥ 3")
    );
    assert_eq!(
        tetgen.set_facet_npoint(5, 3).err(),
        Some("index of facet is out of bounds")
    );
    Ok(())
}

fn set_facet_point_captures_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, None, None, None)?;
    assert_eq!(
        tetgen.set_facet_point(0, 0, 0).err(),
        Some("cannot set facet point because the number of facets is None")
    );
    let mut tetgen = Tetgen::new(4, Some(4), None, None)?;
    assert_eq!(
        tetgen.set_facet_point(5, 0, 0).err(),
        Some("index of facet is out of bounds")
    );
    tetgen.set_facet_npoint(0, 3)?;
    assert_eq!(
        tetgen.set_facet_point(0, 4, 0).err(),
        Some("index of facet point is out of bounds")
    );
    assert_eq!(
        tetgen.set_facet_point(0, 0, 5).err(),
        Some("id of facet point is out of bounds")
    );
    Ok(())
}

fn set_region_captures_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, None, None, None)?;
    assert_eq!(
        tetgen.set_region(0, 0.33, 0.33, 0.33, 1, Some(0.1)).err(),
        Some("cannot set region because the number of regions is None")
    );
    let mut tetgen = Tetgen::new(4, Some(4), Some(1), None)?;
    assert_eq!(
        tetgen.set_region(1, 0.33, 0.33, 0.33, 1, Some(0.1)).err(),
        Some("index of region is out of bounds")
    );
    Ok(())
}

fn set_hole_captures_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, None, None, None)?;
    assert_eq!(
        tetgen.set_hole(0, 0.33, 0.33, 0.33).err(),
        Some("cannot set hole because the number of holes is None")
    );
    let mut tetgen = Tetgen::new(4, Some(4), Some(1), Some(1))?;
    assert_eq!(
        tetgen.set_hole(1, 0.33, 0.33, 0.33).err(),
        Some("index of hole is out of bounds")
    );
    Ok(())
}

fn generate_methods_capture_some_errors() -> Result<(), StrError> {
    let mut tetgen = Tetgen::new(4, Some(4), None, None)?;
    assert_eq!(
        tetgen.generate_delaunay(false).err(),
        Some("cannot generate Delaunay tetrahedralization because not all points are set")
    );
    assert_eq!(
        tetgen.generate_mesh(false, false, None, None).err(),
        Some("cannot generate mesh of tetrahedra because not all points are set")
    );
    tetgen
        .set_point(0, 0.0, 0.0, 0.0)?
        .set_point(1, 1.0, 0.0, 0.0)?
        .set_point(2, 0.0, 1.0, 0.0)?
        .set_point(3, 0.0, 0.0, 1.0)?;
    assert_eq!(
        tetgen.generate_mesh(false, false, None, None).err(),
        Some("cannot generate mesh of tetrahedra because not all facets are set")
    );
    Ok(())
}
