
            class RemoteWebDriverWithExpectations {
                RemoteWebDriverWithExpectations(
                    URL remoteAddress, Capabilities capabilities, List<String> ignoredCommands = DEFAULT_IGNORED_COMMANDS)
                    throws Exception, OtherException
                {
                    super(remoteAddress, capabilities)
                    this.ignoredCommands = ignoredCommands
                }
            }
        