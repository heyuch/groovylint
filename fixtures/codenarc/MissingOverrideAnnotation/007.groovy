
            import java.util.concurrent.Callable;

            class OuterClassWithViolation implements Runnable {

                static class InnerClassWithViolation {
                    void run() {
                    }
                }

                @Override
                void run() {
                }

            }
        