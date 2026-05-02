Feature: Navigation
  Basic browser navigation operations.

  Scenario: Navigate to a page
    Given I navigate to "https://example.org"
    Then the page title should contain "Example"
    And the URL should contain "example.org"

  Scenario: Navigate and check URL
    Given I navigate to "https://example.org"
    Then the URL should be "https://example.org/"

  Scenario: Reload page
    Given I navigate to "https://example.org"
    When I reload the page
    Then the page title should contain "Example"
