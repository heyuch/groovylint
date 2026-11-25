
            class InputStreamsAreCloseable extends InputStream { // through class
                void close() {}
            }

            class ChannelsAreCloseable implements java.nio.channels.Channel { // through interface
                void close() {}
                boolean isOpen() { false }
            }
        