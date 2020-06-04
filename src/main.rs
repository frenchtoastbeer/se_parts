use dirs;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs;
use structopt::StructOpt;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "se_parts",
    about = "Creates a parts list from a space engineers blueprint."
)]
struct Opt {
    /// Space engineers blocks directory
    #[structopt(
        long,
        default_value = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SpaceEngineers\\Content\\Data\\CubeBlocks"
    )]
    blocks_directory: String,

    /// Blueprints directory, relative to your users home folder
    #[structopt(
        long,
        default_value = "\\AppData\\Roaming\\SpaceEngineers\\Blueprints\\local\\"
    )]
    blueprints_directory: String,

    /// Blueprint to check
    #[structopt(short, long)]
    blueprint_name: String,

    /// Report blocks with recipies from the blueprint, helpful for identifying issues with block count
    #[structopt(short, long)]
    report: bool,
}

fn load_recipies(
    blockfile: &str,
    mut block_recipies: HashMap<String, HashMap<String, i64>>,
) -> HashMap<String, HashMap<String, i64>> {
    // load the components recipie for all blocks in existence
    let mut component_reader = Reader::from_file(blockfile).unwrap();
    component_reader.trim_text(true);
    let mut component_buf = Vec::new();

    #[allow(non_snake_case)]
    let mut in_SubtypeId = false;

    #[allow(non_snake_case)]
    let mut in_Components = false;

    #[allow(non_snake_case)]
    let mut in_TypeId = false;

    #[allow(non_snake_case)]
    let mut in_ProducedGases = false;

    #[allow(non_snake_case)]
    let mut current_SubtypeId = String::new();

    #[allow(non_snake_case)]
    let mut current_TypeId = String::new();
    loop {
        match component_reader.read_event(&mut component_buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"SubtypeId" => {
                    if !in_Components && !in_ProducedGases {
                        in_SubtypeId = true;
                    }
                },
                b"Components" => {
                    in_Components = true;
                },
                b"ProducedGases" => {
                    in_ProducedGases = true;
                },
                b"Component" => {
                    let components = e
                        .attributes()
                        .map(|a| {
                            a.unwrap()
                                .unescape_and_decode_value(&component_reader)
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    block_recipies = add_component(
                        block_recipies,
                        &current_SubtypeId,
                        &components[0],
                        components[1].parse::<i64>().unwrap(),
                    );
                }
                b"TypeId" => {
                    if !in_Components&& !in_ProducedGases {
                        in_TypeId = true;
                        current_SubtypeId = "".to_string();
                    }
                }
                _ => (),
            },
            Ok(Event::End(ref e)) => match e.name() {
                b"SubtypeId" => {
                    in_SubtypeId = false;
                    if current_SubtypeId.is_empty() {
                        current_SubtypeId = current_TypeId.clone();
                    }
                },
                b"ProducedGases" => {
                    in_ProducedGases = false;
                },
                b"Components" => {
                    in_Components = false;
                }
                b"TypeId" => {
                    in_TypeId = false;
                }
                _ => (),
            },
            Ok(Event::Text(e)) => {
                if in_TypeId {
                    current_TypeId = e.unescape_and_decode(&component_reader).unwrap();
                }
                if in_SubtypeId {
                    current_SubtypeId = e.unescape_and_decode(&component_reader).unwrap();
                }
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Ok(Event::Empty(e)) => {
                // Sometimes SubtypeId is empty, if so use the TypeId instead
                if String::from_utf8_lossy(e.name()) == "SubtypeId" {
                    current_SubtypeId = current_TypeId.clone();
                }
                if in_Components {
                    let components = e
                        .attributes()
                        .map(|a| {
                            a.unwrap()
                                .unescape_and_decode_value(&component_reader)
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    block_recipies = add_component(
                        block_recipies,
                        &current_SubtypeId,
                        &components[0],
                        components[1].parse::<i64>().unwrap(),
                    );
                }
            }
            Err(e) => panic!(
                "Error at position {}: {:?}",
                component_reader.buffer_position(),
                e
            ),
            _ => (), // There are several other `Event`s we do not consider here
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        component_buf.clear();
    }
    block_recipies
}

fn add_component(
    mut block_recipies: HashMap<String, HashMap<String, i64>>,
    block: &String,
    component: &String,
    ammount: i64,
) -> HashMap<String, HashMap<String, i64>> {
    if !block_recipies.contains_key(block) {
        block_recipies.insert(block.clone(), HashMap::new());
    }
    if block_recipies.get(block).unwrap().contains_key(component) {
        let prior_value = block_recipies
            .get(block)
            .unwrap()
            .get(component)
            .unwrap()
            .clone();
        block_recipies
            .get_mut(block)
            .unwrap()
            .insert(component.clone(), prior_value + ammount);
    } else {
        block_recipies
            .get_mut(block)
            .unwrap()
            .insert(component.clone(), ammount);
    }
    block_recipies
}

fn main() {
    let opt = Opt::from_args();
    let home_dir = dirs::home_dir().unwrap();
    let blueprints_dir = opt.blueprints_directory;
    let blueprint_file = format!(
        "{}{}{}\\bp.sbc",
        home_dir.to_str().unwrap(),
        blueprints_dir,
        opt.blueprint_name
    );
    let mut reader = Reader::from_file(blueprint_file).unwrap();
    reader.trim_text(true);

    // load vanilla block recipies
    let mut block_recipies = HashMap::new();
    let paths = fs::read_dir(opt.blocks_directory).unwrap();
    for path in paths {
        let path = format!("{}", path.unwrap().path().display());
        block_recipies = load_recipies(
            &format!("{}", path),
            block_recipies,
        );
    }

    // for (block, recipie) in &block_recipies {
    //     if block.contains("Oxygen") {
    //         println!("{}:{:#?}", block, recipie);
    //     }
    // }

    let mut count = 0;
    let mut buf = Vec::new();
    let mut blocks = Vec::new();

    #[allow(non_snake_case)]
    let mut in_SubTypeName = false;

    let mut missed_blocks = HashSet::new();
    let mut hit_blocks = HashMap::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"SubtypeName" => {
                    in_SubTypeName = true;
                },
                b"MyObjectBuilder_CubeBlock" => {
                    let components = e
                        .attributes()
                        .map(|a| {
                            a.unwrap()
                                .unescape_and_decode_value(&reader)
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    let value = components.get(0);
                    if value.is_some() { 
                        let xsitype = value.unwrap().to_string();
                        if xsitype == "MyObjectBuilder_OxygenGenerator" {
                            let potential_block = "OxygenGenerator".to_string();
                            if !block_recipies.contains_key(&potential_block) {
                                missed_blocks.insert(potential_block.clone());
                            } else {
                                if hit_blocks.contains_key(&potential_block) {
                                    hit_blocks.insert(potential_block.clone(), hit_blocks.get(&potential_block).unwrap() + 1);
                                } else {
                                    hit_blocks.insert(potential_block.clone(), 1);
                                }
                                blocks.push(potential_block);
                                count += 1;
                            }
                        }
                    }
                },
                _ => (),
            },
            Ok(Event::End(ref e)) => match e.name() {
                b"SubtypeName" => {
                    in_SubTypeName = false;
                },
                _ => (),
            },
            // Ok(Event::Empty(e)) => {
            //     // Sometimes SubtypeId is empty, if so use the TypeId instead
            //     if String::from_utf8_lossy(e.name()) == "SubtypeId" {
            //         if last_component == "MyObjectBuilder_OxygenGenerator" {

            //         }
            //     }
            // },
            Ok(Event::Text(e)) => {
                let mut potential_block = e.unescape_and_decode(&reader).unwrap();
                // name corrections for block SubtypeID mis-alignment
                // MyObjectBuilder_OxygenGenerator
                if potential_block == "WideLargeCameraBlock" {
                    potential_block = "LargeCameraBlock".to_string();
                }
                if potential_block == "LargeBlockLargeDrill" {
                    potential_block = "LargeBlockDrill".to_string();
                }
                if in_SubTypeName && !block_recipies.contains_key(&potential_block) {
                    missed_blocks.insert(potential_block.clone());
                }
                if in_SubTypeName && block_recipies.contains_key(&potential_block) {
                    if hit_blocks.contains_key(&potential_block) {
                        hit_blocks.insert(potential_block.clone(), hit_blocks.get(&potential_block).unwrap() + 1);
                    } else {
                        hit_blocks.insert(potential_block.clone(), 1);
                    }
                    blocks.push(potential_block);
                    count += 1;
                }
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }

    // Count the totals for all the blocks detected in the grid
    let mut all_components: HashMap<String, i64> = HashMap::new();
    for block in blocks {
        let components = block_recipies.get_mut(&block).unwrap();
        for (component, count) in components.iter_mut() {
            if all_components.contains_key(component) {
                let prior_count = all_components.get(component).unwrap().clone();
                all_components.insert(component.clone(), prior_count + count.clone());
            } else {
                all_components.insert(component.clone(), count.clone());
            }
        }
    }

    if opt.report {
        println!("Blocks without recipeies:\n{:#?}", missed_blocks);
        println!("Blocks with recipies:\n{:#?}", hit_blocks);
    } else {
        println!("Component totals:\n{:#?}", &all_components);
        println!("{} blocks with matching component recipies", count);
    }
}
