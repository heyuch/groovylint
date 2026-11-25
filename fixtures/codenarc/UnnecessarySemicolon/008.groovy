
            import java.net.*;
            import java.lang.String;

            import org.SomeOther                    // no violation
            import org.util.*                       // no violation
            import static org.SomeUtil.*            // no violation
            import static org.SomeUtil.doStuff      // no violation

            import static java.lang.Math.*;
            import static org.other.OtherUtil.doStuff;
        