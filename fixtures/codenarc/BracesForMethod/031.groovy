
            class RemoteWebDriverWithExpectations {
                RemoteWebDriverWithExpectations(
                    URL remoteAddress, Capabilities capabilities, List<String> ignoredCommands = DEFAULT_IGNORED_COMMANDS)
                    throws Exception, OtherException
                { // Some comment
                    super(remoteAddress, capabilities)
                    this.ignoredCommands = ignoredCommands
                }
            }
        