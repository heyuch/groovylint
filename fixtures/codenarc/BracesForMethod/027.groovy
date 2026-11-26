
            class RemoteWebDriverWithExpectations {
                RemoteWebDriverWithExpectations(
                    URL remoteAddress, Capabilities capabilities, List<String> ignoredCommands = DEFAULT_IGNORED_COMMANDS)
                { // Some comment
                    super(remoteAddress, capabilities)
                    this.ignoredCommands = ignoredCommands
                }
            }
        