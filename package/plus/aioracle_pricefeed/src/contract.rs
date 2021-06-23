use aioracle::create_contract_with_aggregate;
use cosmwasm_std::StdResult;

create_contract_with_aggregate!(aggregate);

pub fn aggregate(results: &[String]) -> StdResult<String> {
    let mut sum: i32 = 0;
    let mut floating_sum: i32 = 0;
    let mut count = 0;
    for result in results {
        // get first item from iterator
        let mut iter = result.split('.');
        let first = iter.next();
        let last = iter.next();
        // will panic instead for forward error with ?
        let number: i32 = first.unwrap().parse().unwrap_or(0);
        let mut floating: i32 = 0;
        if last.is_some() {
            let mut last_part = last.unwrap().to_owned();
            if last_part.len() < 2 {
                last_part.push_str("0");
            } else if last_part.len() > 2 {
                last_part = last_part[..2].to_string();
            }
            floating = last_part.parse().unwrap_or(0);
        }
        sum += number;
        floating_sum += floating;
        count += 1;
    }

    let mut final_result = String::new();
    // has results found, update report
    if count > 0 {
        sum = sum / count;
        floating_sum = floating_sum / count;
        final_result = format!("{}.{}", sum, floating_sum);
    }

    Ok(final_result)
}
