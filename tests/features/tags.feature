@smoke
Feature: Tag Filtering
  Tests for tag-based scenario filtering.

  @fast
  Scenario: Fast smoke test
    Given I navigate to "https://example.org"
    Then "h1" should be visible

  @slow
  Scenario: Slow test with wait
    Given I navigate to "https://example.org"
    When I wait 1 seconds
    Then the page title should contain "Example"

  @skip
  Scenario: Skipped scenario
    Given I navigate to "https://nonexistent.example.org"
    Then "h1" should be visible
