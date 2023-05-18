use owo_colors::OwoColorize;
use prettytable::{row, Table};

use crate::{
    data::{
        modinfo::ModInfoFull, Cacheable, CompoundKey, EndorsementList, EndorsementStatus,
        GameMetadata, UserEndorsement,
    },
    formatting::pluralize_mod,
    nexus::NexusClient,
    Flags,
};

/// Display mod endorsements for a specific game, sorted by status.
fn show_endorsements(
    game: &str,
    modlist: &[UserEndorsement],
    store: &kv::Store,
    client: &mut NexusClient,
) {
    let game_meta = GameMetadata::get(&game.to_string(), false, store, client).unwrap();
    println!(
        "\n{} opinions for {}",
        pluralize_mod(modlist.len()),
        game_meta.name().yellow().bold()
    );
    // I think there's a split function I could use instead.
    let abstained: Vec<&UserEndorsement> = modlist
        .iter()
        .filter(|m| matches!(m.status(), EndorsementStatus::Abstained))
        .collect();
    let endorsed: Vec<&UserEndorsement> = modlist
        .iter()
        .filter(|m| !matches!(m.status(), EndorsementStatus::Abstained))
        .collect();

    let mut emit_table = |list: Vec<&UserEndorsement>| {
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
        list.iter().for_each(|opinion| {
            let key = CompoundKey::new(game.to_string(), opinion.mod_id());
            if let Some(mod_info) = ModInfoFull::get(&key, false, store, client) {
                table.add_row(row![
                    format!("{}", opinion.status()),
                    format!(
                        "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
                        opinion.url(),
                        mod_info.display_name()
                    ),
                ]);
            } else {
                table.add_row(row![
                    format!("{}", opinion.status()),
                    format!(
                        "\x1b]8;;{}\x1b\\uncached mod id #{}\x1b]8;;\x1b\\",
                        opinion.url(),
                        opinion.mod_id()
                    ),
                ]);
            }
        });
        println!("{table}");
    };

    println!("endorsed {}:", pluralize_mod(endorsed.len()));
    emit_table(endorsed);
    println!("abstained on {}:", pluralize_mod(abstained.len()));
    emit_table(abstained);
}

pub fn handle(flags: &Flags, game: &Option<String>, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();

    let maybe: Option<Box<EndorsementList>> =
        EndorsementList::get(&EndorsementList::listkey(), flags.refresh, store, nexus);

    let Some(opinions) = maybe else {
        log::error!("Something went wrong fetching endorsements. Rerun with -v to get more details.");
        return Ok(());
    };
    let mapping = opinions.get_game_map();

    if let Some(game) = game {
        if let Some(modlist) = mapping.get(game) {
            show_endorsements(game, modlist, store, nexus);
        } else {
            println!("No opinions expressed on mods for {}.", game);
        }
    } else {
        if flags.json {
            let pretty = serde_json::to_string_pretty(&opinions)?;
            println!("{}", pretty);
        } else {
            println!(
                "\n{} mods opinionated upon for {} games\n",
                opinions.mods.len().red(),
                mapping.len().blue()
            );
            for (game, modlist) in mapping.iter() {
                show_endorsements(game, modlist, store, nexus);
            }
        }
        return Ok(());
    }

    Ok(())
}
