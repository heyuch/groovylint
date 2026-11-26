
            class RemoteWebDriverWithExpectations {
                RemoteWebDriverWithExpectations(
                    URL remoteAddress, Capabilities capabilities, List<String> ignoredCommands = DEFAULT_IGNORED_COMMANDS)
                {
                    super(remoteAddress, capabilities)
                    this.ignoredCommands = ignoredCommands
                }
            }
        