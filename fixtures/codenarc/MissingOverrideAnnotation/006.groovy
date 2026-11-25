
            import java.util.concurrent.Callable;

            class OuterClassWithViolation implements Callable<String> {

                static class InnerClassWithViolation implements Runnable {
                    void run() {
                    }
                }

                String call() {
                }

            }
        