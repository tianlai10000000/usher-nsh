/** Occupies the test key before startup to exercise real native rejection and explicit recovery. */
import { mount } from "nsh";
import { fixture } from "./fixture";
import { Content } from "./Content";
if (!fixture.reserve()) throw new Error("test_global_key_already_occupied");
mount(Content);
