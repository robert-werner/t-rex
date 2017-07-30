//
// Copyright (c) Pirmin Kalberer. All rights reserved.
// Licensed under the MIT License. See LICENSE file in the project root for full license information.
//

use datasource::DatasourceInput;
use datasource::postgis::{PostgisInput, QueryParam};
use postgres;
use postgres::Connection;
use core::feature::FeatureAttrValType;
use core::geom::*;
use core::grid::Extent;
use core::grid::Grid;
use core::layer::{Layer, LayerQuery};
use std::env;


#[test]
#[ignore]
fn test_from_geom_fields() {
    let conn: Connection = match env::var("DBCONN") {
            Result::Ok(val) => Connection::connect(&val as &str, postgres::TlsMode::None),
            Result::Err(_) => panic!("DBCONN undefined"),
        }
        .unwrap();
    let sql = "SELECT wkb_geometry FROM ne_10m_populated_places LIMIT 1";
    for row in &conn.query(sql, &[]).unwrap() {
        let geom = row.get::<_, Point>("wkb_geometry");
        assert_eq!(&*format!("{:?}", geom),
                   "Point { x: -6438719.622820721, y: -4093437.7144101723, srid: Some(3857) }");
        let geom = GeometryType::from_geom_field(&row, "wkb_geometry", "POINT");
        assert_eq!(&*format!("{:?}", geom),
                   "Ok(Point(Point { x: -6438719.622820721, y: -4093437.7144101723, srid: Some(3857) }))");
    }

    let sql = "SELECT ST_Multi(wkb_geometry) AS wkb_geometry FROM rivers_lake_centerlines WHERE name='Waiau' AND ST_NPoints(wkb_geometry)<10";
    for row in &conn.query(sql, &[]).unwrap() {
        let geom = GeometryType::from_geom_field(&row, "wkb_geometry", "LINESTRING");
        assert_eq!(&*format!("{:?}", geom),
                   "Ok(MultiLineString(MultiLineStringT { lines: [LineStringT { points: [Point { x: 18672061.098933436, y: -5690573.725394946, srid: None }, Point { x: 18671798.382036217, y: -5692123.11701991, srid: None }, Point { x: 18671707.790002696, y: -5693530.713572942, srid: None }, Point { x: 18671789.322832868, y: -5694822.281317252, srid: None }, Point { x: 18672061.098933436, y: -5695997.770001522, srid: None }, Point { x: 18670620.68560042, y: -5698245.837796968, srid: None }, Point { x: 18668283.41113552, y: -5700403.997584983, srid: None }, Point { x: 18666082.024720907, y: -5701179.511527114, srid: None }, Point { x: 18665148.926775623, y: -5699253.775757339, srid: None }], srid: None }], srid: Some(3857) }))");
    }
    let sql = "SELECT wkb_geometry FROM ne_10m_rivers_lake_centerlines WHERE name='Belaya' AND ST_NPoints(wkb_geometry)<10";
    for row in &conn.query(sql, &[]).unwrap() {
        let geom = row.get::<_, MultiLineString>("wkb_geometry");
        assert_eq!(&*format!("{:?}", geom),
                   "MultiLineStringT { lines: [LineStringT { points: [Point { x: 5959308.212236793, y: 7539958.36540974, srid: None }, Point { x: 5969998.072192525, y: 7539958.36540974, srid: None }, Point { x: 5972498.412317764, y: 7539118.002915677, srid: None }, Point { x: 5977308.849297845, y: 7535385.962035617, srid: None }], srid: None }], srid: Some(3857) }");
    }
    let sql = "SELECT wkb_geometry, ST_AsBinary(wkb_geometry) FROM rivers_lake_centerlines LIMIT 1";
    let rows = &conn.query(sql, &[]).unwrap();
    assert_eq!(rows.columns()[0].name(), "wkb_geometry");
    assert_eq!(format!("{}", rows.columns()[0].type_()), "geometry");
    assert_eq!(rows.columns()[1].name(), "st_asbinary");
    assert_eq!(format!("{}", rows.columns()[1].type_()), "bytea");
}

#[test]
#[ignore]
fn test_detect_layers() {
    let pg: PostgisInput = match env::var("DBCONN") {
            Result::Ok(val) => Some(PostgisInput::new(&val).connected()),
            Result::Err(_) => panic!("DBCONN undefined"),
        }
        .unwrap();
    let layers = pg.detect_layers(false);
    assert!(layers
                .iter()
                .any(|ref layer| layer.name == "rivers_lake_centerlines"));
}

#[test]
#[ignore]
fn test_detect_columns() {
    let pg: PostgisInput = match env::var("DBCONN") {
            Result::Ok(val) => Some(PostgisInput::new(&val).connected()),
            Result::Err(_) => panic!("DBCONN undefined"),
        }
        .unwrap();
    let layers = pg.detect_layers(false);
    let layer = layers
        .iter()
        .find(|ref layer| layer.name == "rivers_lake_centerlines")
        .unwrap();
    let cols = pg.detect_data_columns(&layer, None);
    assert_eq!(cols,
               vec![("fid".to_string(), "".to_string()),
                    ("scalerank".to_string(), "FLOAT8".to_string()),
                    ("name".to_string(), "".to_string())]);
}

#[test]
#[ignore]
fn test_extent_query() {
    let pg: PostgisInput = match env::var("DBCONN") {
            Result::Ok(val) => Some(PostgisInput::new(&val).connected()),
            Result::Err(_) => panic!("DBCONN undefined"),
        }
        .unwrap();
    let layers = pg.detect_layers(false);
    let layer = &layers
                     .iter()
                     .find(|ref layer| layer.name == "rivers_lake_centerlines")
                     .unwrap();
    assert_eq!(pg.layer_extent(&layer),
               Some(Extent {
                        minx: -164.90347246002037,
                        miny: -52.1577287739643,
                        maxx: 177.2111922535212,
                        maxy: 75.79348379113983,
                    }));
}

#[test]
fn test_feature_query() {
    let pg = PostgisInput::new("postgresql://pi@localhost/osm2vectortiles");
    let mut layer = Layer::new("points");
    layer.table_name = Some(String::from("osm_place_point"));
    layer.geometry_field = Some(String::from("geometry"));
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_SetSRID(geometry,3857) AS geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    // reprojection
    layer.srid = Some(2056);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_Transform(geometry,3857) AS geometry FROM osm_place_point WHERE geometry && ST_Transform(ST_MakeEnvelope($1,$2,$3,$4,3857),2056)");
    layer.srid = Some(-1);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_SetSRID(geometry,3857) AS geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,-1)");
    layer.srid = Some(3857);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    // clipping
    layer.buffer_size = Some(10);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_Intersection(ST_MakeValid(geometry),ST_Buffer(ST_MakeEnvelope($1,$2,$3,$4,3857),10*$5::FLOAT8)) AS geometry FROM osm_place_point WHERE geometry && ST_Buffer(ST_MakeEnvelope($1,$2,$3,$4,3857),10*$5::FLOAT8)");
    layer.geometry_type = Some("POLYGON".to_string());
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_Multi(ST_Buffer(ST_Intersection(ST_MakeValid(geometry),ST_Buffer(ST_MakeEnvelope($1,$2,$3,$4,3857),10*$5::FLOAT8)), 0.0)) AS geometry FROM osm_place_point WHERE geometry && ST_Buffer(ST_MakeEnvelope($1,$2,$3,$4,3857),10*$5::FLOAT8)");
    layer.geometry_type = Some("POINT".to_string());
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT geometry FROM osm_place_point WHERE geometry && ST_Buffer(ST_MakeEnvelope($1,$2,$3,$4,3857),10*$5::FLOAT8)");
    layer.buffer_size = Some(0);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    layer.buffer_size = None;
    layer.geometry_type = Some("POLYGON".to_string());

    // simplification
    layer.simplify = Some(true);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT COALESCE(ST_SnapToGrid(ST_Multi(geometry), $5::FLOAT8/2),ST_GeomFromText('MULTIPOLYGON EMPTY',3857))::geometry(MULTIPOLYGON,3857) AS geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");
    layer.geometry_type = Some("LINESTRING".to_string());
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT ST_Multi(ST_SimplifyPreserveTopology(ST_Multi(geometry),$5::FLOAT8/2)) AS geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");
    layer.geometry_type = Some("POINT".to_string());
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               "SELECT geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    layer.simplify = Some(false);
    layer.query_limit = Some(1);
    assert_eq!(pg.build_query(&layer, 3857, None).unwrap().sql,
               // No LIMIT clause added - limited when retrieving records
               "SELECT geometry FROM osm_place_point WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    // user queries
    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT geometry AS geom FROM osm_place_point")),
                       }];
    layer.query_limit = None;
    assert_eq!(pg.build_query(&layer, 3857, layer.query[0].sql.as_ref())
                   .unwrap()
                   .sql,
               "SELECT * FROM (SELECT geometry AS geom FROM osm_place_point) AS _q WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT * FROM osm_place_point WHERE name='Bern'")),
                       }];
    assert_eq!(pg.build_query(&layer, 3857, layer.query[0].sql.as_ref())
                   .unwrap()
                   .sql,
               "SELECT * FROM (SELECT * FROM osm_place_point WHERE name='Bern') AS _q WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");

    // out of maxzoom
    //assert_eq!(pg.query(&layer, 23).unwrap().sql,
    //    "SELECT * FROM (SELECT geometry FROM osm_place_point) AS _q WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)");
    //layer.table_name = None;
    //assert!(pg.query(&layer, 23).is_none());
}

#[test]
fn test_query_params() {
    let pg = PostgisInput::new("postgresql://pi@localhost/osm2vectortiles");
    let mut layer = Layer::new("buildings");
    layer.geometry_field = Some(String::from("way"));

    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT name, type, 0 as osm_id, ST_Union(geometry) AS way FROM osm_buildings_gen0 WHERE geometry && !bbox!")),
                       }];
    let query = pg.build_query(&layer, 3857, layer.query[0].sql.as_ref())
        .unwrap();
    assert_eq!(query.sql,
               "SELECT * FROM (SELECT name, type, 0 as osm_id, ST_Union(geometry) AS way FROM osm_buildings_gen0 WHERE geometry && ST_MakeEnvelope($1,$2,$3,$4,3857)) AS _q");
    assert_eq!(query.params, [QueryParam::Bbox]);

    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT osm_id, geometry, typen FROM landuse_z13toz14n WHERE !zoom! BETWEEN 13 AND 14) AS landuse_z9toz14n")),
                       }];
    let query = pg.build_query(&layer, 3857, layer.query[0].sql.as_ref())
        .unwrap();
    assert_eq!(query.sql,
               "SELECT * FROM (SELECT osm_id, geometry, typen FROM landuse_z13toz14n WHERE $5 BETWEEN 13 AND 14) AS landuse_z9toz14n) AS _q WHERE way && ST_MakeEnvelope($1,$2,$3,$4,3857)");
    assert_eq!(query.params, [QueryParam::Bbox, QueryParam::Zoom]);

    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT name, type, 0 as osm_id, ST_SimplifyPreserveTopology(ST_Union(geometry),!pixel_width!/2) AS way FROM osm_buildings")),
                       }];
    let query = pg.build_query(&layer, 3857, layer.query[0].sql.as_ref())
        .unwrap();
    assert_eq!(query.sql,
               "SELECT * FROM (SELECT name, type, 0 as osm_id, ST_SimplifyPreserveTopology(ST_Union(geometry),$5::FLOAT8/2) AS way FROM osm_buildings) AS _q WHERE way && ST_MakeEnvelope($1,$2,$3,$4,3857)");
    assert_eq!(query.params, [QueryParam::Bbox, QueryParam::PixelWidth]);
}

#[test]
#[ignore]
fn test_retrieve_features() {
    let mut pg: PostgisInput = match env::var("DBCONN") {
            Result::Ok(val) => Some(PostgisInput::new(&val).connected()),
            Result::Err(_) => panic!("DBCONN undefined"),
        }
        .unwrap();

    let mut layer = Layer::new("points");
    layer.table_name = Some(String::from("ne_10m_populated_places"));
    layer.geometry_field = Some(String::from("wkb_geometry"));
    layer.geometry_type = Some(String::from("POINT"));
    let grid = Grid::web_mercator();
    let extent = Extent {
        minx: 821850.9,
        miny: 5909499.5,
        maxx: 860986.7,
        maxy: 5948635.3,
    };

    let mut reccnt = 0;
    pg.prepare_queries(&layer, 3857);
    pg.retrieve_features(&layer, &extent, 10, &grid, |feat| {
        assert_eq!("Ok(Point(Point { x: 831219.9062494118, y: 5928485.165733484, srid: Some(3857) }))",
                   &*format!("{:?}", feat.geometry()));
        assert_eq!(4, feat.attributes().len());
        assert_eq!(None, feat.fid());
        reccnt += 1;
    });
    assert_eq!(1, reccnt);

    layer.query = vec![LayerQuery {
                           minzoom: Some(0),
                           maxzoom: Some(22),
                           sql: Some(String::from("SELECT * FROM ne_10m_populated_places")),
                       }];
    layer.fid_field = Some(String::from("fid"));
    pg.prepare_queries(&layer, 3857);
    pg.retrieve_features(&layer, &extent, 10, &grid, |feat| {
        assert_eq!("Ok(Point(Point { x: 831219.9062494118, y: 5928485.165733484, srid: Some(3857) }))",
                   &*format!("{:?}", feat.geometry()));
        assert_eq!(feat.attributes()[0].key, "fid");
        assert_eq!(feat.attributes()[1].key, "scalerank"); //Numeric
        assert_eq!(feat.attributes()[2].key, "name");
        assert_eq!(feat.attributes()[3].key, "pop_max"); //Numeric
        assert_eq!(feat.attributes()[0].value, FeatureAttrValType::Int(6478));
        assert_eq!(feat.attributes()[1].value, FeatureAttrValType::Double(4.0));
        assert_eq!(feat.attributes()[2].value,
                   FeatureAttrValType::String("Bern".to_string()));
        assert_eq!(feat.fid(), Some(6478));
    });

}

#[test]
#[ignore]
fn test_tls() {
    use postgres::TlsMode;
    use postgres::tls::native_tls::NativeTls;
    let negotiator = NativeTls::new().unwrap();
    let conn = match env::var("DBCONN") {
            Result::Ok(val) => Connection::connect(&val as &str, TlsMode::Prefer(&negotiator)),
            Result::Err(_) => panic!("DBCONN undefined"),
        };
    assert!(conn.is_ok());
    assert!(conn.unwrap().execute("SELECT 1::VARCHAR", &[]).is_ok());
    // Check pg_stat_ssl? https://www.postgresql.org/docs/9.6/static/monitoring-stats.html#PG-STAT-SSL-VIEW
}
