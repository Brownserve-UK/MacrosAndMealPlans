use super::*;

#[test]
fn page_requests_are_clamped() {
    let request = PageRequest::new(0, 10_000);
    assert_eq!(request.page(), 1);
    assert_eq!(request.per_page(), PageRequest::MAX_PER_PAGE);
}

#[test]
fn offsets_are_zero_based() {
    assert_eq!(PageRequest::new(1, 25).offset(), 0);
    assert_eq!(PageRequest::new(3, 25).offset(), 50);
}

#[test]
fn total_pages_rounds_up() {
    let page: Paginated<()> = Paginated::new(vec![], 51, PageRequest::new(1, 25));
    assert_eq!(page.total_pages(), 3);
}

#[test]
fn an_empty_result_has_no_pages() {
    let page: Paginated<()> = Paginated::new(vec![], 0, PageRequest::default());
    assert_eq!(page.total_pages(), 0);
}
